mod engine;
pub(crate) mod handler;
mod scaffold;

use crate::{
    api::{Exclusive, Inclusive, IntermediateEvent, Task},
    bpmn::Bpmn,
    diagram::{Diagram, reader::read_bpmn},
    process::handler::Callback,
};
use engine::ExecuteInput;
use handler::Handler;
use std::{marker::PhantomData, path::Path, str::FromStr};

/// Process Build state
pub struct Build;

/// Process Run state
pub struct Run;

/// Process that contains information from the BPMN file and registered functions
pub struct Process<T, S = Build>
where
    Self: Sync + Send,
{
    diagram: Diagram,
    handler: Handler<T>,
    _marker: PhantomData<S>,
}

impl<T> Process<T> {
    /// Create new process and initialize it from the BPMN file path.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = Process::new("examples/example.bpmn")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, BpmnFileError> {
        let path = path.as_ref();
        let diagram = (|| {
            let reader = quick_xml::Reader::from_file(path)
                .map_err(|e| BpmnFileErrorKind::ReadFile(e.into()))?;
            read_bpmn(reader).map_err(BpmnFileErrorKind::Parse)
        })()
        .map_err(|source| BpmnFileError {
            path: path.into(),
            source,
        })?;

        Ok(Self {
            diagram,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }

    /// Register a task function with name or bpmn id
    pub fn task<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> Task + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Task(Box::new(func)));
        self
    }

    /// Register an exclusive gateway function with name or bpmn id
    pub fn exclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> Exclusive + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Exclusive(Box::new(func)));
        self
    }

    /// Register an inclusive gateway function with name or bpmn id
    pub fn inclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> Inclusive + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::Inclusive(Box::new(func)));
        self
    }

    /// Register an event based gateway function with name or bpmn id
    pub fn event_based<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> IntermediateEvent + 'static + Sync + Send,
    {
        self.handler
            .add_callback(name, Callback::EventBased(Box::new(func)));
        self
    }

    /// Install and check that all required functions have been registered. You cannot run a process before `build` is called.
    /// If `build` returns an error, it contains the missing functions.
    pub fn build(mut self) -> Result<Process<T, Run>, BuildError> {
        let result = self.diagram.install_and_check(self.handler.build()?);
        if result.is_empty() {
            Ok(Process {
                diagram: self.diagram,
                handler: self.handler,
                _marker: Default::default(),
            })
        } else {
            Err(BuildError::MissingImplementations(
                result.into_iter().collect::<Vec<_>>().join(", "),
            ))
        }
    }
}

impl<T> FromStr for Process<T> {
    type Err = ParseError;

    /// Create new process and initialize it from a BPMN `&str`.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/example.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            diagram: read_bpmn(quick_xml::Reader::from_str(s))?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl<T> Process<T, Run> {
    /// Run the process and return the `T` or an `Error`.
    /// ```
    /// use snurr::Process;
    /// use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter(AtomicU32);
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     // Create process from BPMN file
    ///     let bpmn = Process::<Counter>::new("examples/example.bpmn")?
    ///         .task("Count 1", |input| {
    ///             input.0.fetch_add(1, Relaxed);
    ///             Default::default()
    ///         })
    ///         .exclusive("equal to 3", |input| {
    ///             match input.0.load(Relaxed) {
    ///                 3 => "YES",
    ///                 _ => "NO",
    ///             }
    ///             .into()
    ///         })
    ///         .build()?;
    ///
    ///     // Run the process with input data
    ///     let result = bpmn.run(Default::default())?;
    ///
    ///     // Print the result.
    ///     println!("{result:?}");
    ///     Ok(())
    /// }
    /// ```
    pub fn run(&self, data: T) -> Result<T, RuntimeError>
    where
        T: Send + Sync,
    {
        // Run every process specified in the diagram
        for bpmn in self
            .diagram
            .get_definition()
            .ok_or(RuntimeErrorKind::MissingDefinitionsId)?
            .iter()
        {
            if let Bpmn::Process {
                id,
                data_index: Some(index),
                ..
            } = bpmn
            {
                let process_data = self
                    .diagram
                    .get_process(*index)
                    .ok_or_else(|| RuntimeErrorKind::MissingProcessData(id.bpmn().into()))?;
                self.execute(ExecuteInput::new(process_data, false, &data))?;
            }
        }
        Ok(data)
    }
}

// Process errors

#[derive(thiserror::Error, Debug)]
#[error("error reading `{path}`")]
#[non_exhaustive]
pub struct BpmnFileError {
    pub path: Box<std::path::Path>,
    pub source: BpmnFileErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum BpmnFileErrorKind {
    ReadFile(Box<dyn std::error::Error + Send + Sync>),
    Parse(ParseError),
}

/// Errors that can occur while parsing BPMN data.
#[derive(thiserror::Error, Debug)]
#[error("error parsing `{source}`")]
#[non_exhaustive]
pub struct ParseError {
    #[from]
    pub source: ParseErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseErrorKind {
    #[error("BPMN type {0} missing id")]
    MissingId(String),
    #[error("sequenceFlow missing targetRef")]
    MissingTargetRef,
    #[error("type {0} not implemented")]
    TypeNotImplemented(String),
    #[error("{0} not supported")]
    NotSupported(String),
    #[error("could not build process")]
    ProcessBuild,
    #[error(transparent)]
    Encoding(Box<dyn std::error::Error + Send + Sync>),
    #[error("Error at position {pos} with {source}")]
    Xml {
        pos: u64,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl From<std::str::Utf8Error> for ParseError {
    fn from(value: std::str::Utf8Error) -> Self {
        ParseError {
            source: ParseErrorKind::Encoding(value.into()),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("error running `{source}`")]
#[non_exhaustive]
pub struct RuntimeError {
    #[from]
    pub source: RuntimeErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum RuntimeErrorKind {
    #[error("{0} has no output. (Used correct name or id?)")]
    MissingOutput(String),
    #[error("{0} has no implementation")]
    MissingImplementation(String),
    #[error("{0} has no default flow")]
    MissingDefault(String),
    #[error("could not find BPMN data with id {0}")]
    MisssingBpmnData(String),
    #[error("could not find process data with id {0}")]
    MissingProcessData(String),
    #[error("missing definitions id")]
    MissingDefinitionsId,
    #[error("type {0} not implemented")]
    TypeNotImplemented(String),
    #[error("could not find {0} boundary symbol attached to {1}")]
    MissingBoundary(String, String),
    #[error("{0} could not find {1}")]
    MissingIntermediateEvent(String, String),
    #[error("missing intermediate throw event name on {0}")]
    MissingIntermediateThrowEventName(String),
    #[error("missing intermediate catch event symbol {0} with name {1}")]
    MissingIntermediateCatchEvent(String, String),
    #[error("missing end event")]
    MissingEndEvent,
    #[error("missing start event")]
    MissingStartEvent,
    #[error("{0} not supported")]
    NotSupported(String),
    #[error("{0}")]
    BpmnRequirement(String),
}

/// Errors that can occur while trying to build a process to make it runnable.
#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Missing implementations {0}")]
    MissingImplementations(String),
    #[error("Handlermap has already been consumed")]
    MapConsumed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?
            .task("Count 1", |_| Default::default())
            .exclusive("equal to 3", |_| Default::default())
            .build()?;
        bpmn.run({})?;
        Ok(())
    }
}
