mod engine;
pub mod handler;
mod scaffold;

use crate::{
    api::{Exclusive, Inclusive, IntermediateEvent, Task},
    bpmn::Bpmn,
    diagram::{Diagram, reader::read_bpmn},
    error::Error,
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
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            diagram: read_bpmn(quick_xml::Reader::from_file(path)?)?,
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
    pub fn build(mut self) -> Result<Process<T, Run>, Error> {
        let result = self.diagram.install_and_check(self.handler.build()?);
        if result.is_empty() {
            Ok(Process {
                diagram: self.diagram,
                handler: self.handler,
                _marker: Default::default(),
            })
        } else {
            Err(Error::MissingImplementations(
                result.into_iter().collect::<Vec<_>>().join(", "),
            ))
        }
    }
}

impl<T> FromStr for Process<T> {
    type Err = Error;

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
    pub fn run(&self, data: T) -> Result<T, Error>
    where
        T: Send + Sync,
    {
        // Run every process specified in the diagram
        for bpmn in self
            .diagram
            .get_definition()
            .ok_or(Error::MissingDefinitionsId)?
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
                    .ok_or_else(|| Error::MissingProcessData(id.bpmn().into()))?;
                self.execute(ExecuteInput::new(process_data, false, &data))?;
            }
        }
        Ok(data)
    }
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
