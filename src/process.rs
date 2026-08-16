mod engine;
pub mod func_map;
pub(crate) mod handler;
mod scaffold;

use crate::{
    Symbol,
    api::{Exclusive, Inclusive, IntermediateEvent, Task},
    bpmn::BpmnType,
    diagram::{
        Diagram,
        reader::{BpmnError, read_bpmn},
    },
    process::{func_map::FuncMap, handler::Callback},
};
use core::fmt;
use engine::ExecuteInput;
use handler::Handler;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::Path,
    str::FromStr,
};

/// Process builder that contains information from the BPMN file and registered functions
pub struct ProcessBuilder<T>
where
    Self: Sync + Send,
{
    diagram: Diagram,
    handler: Handler<T>,
    func_map: FuncMap,
    end_callback: Option<usize>,
    intermediate_throw_callback: Option<usize>,
    intermediate_catch_callback: Option<usize>,
}

impl<T> ProcessBuilder<T> {
    /// Creates a process builder and initialize it from the BPMN file path. Returns an error if
    /// the file was not found or if there were problems with the BPMN content.
    /// ```no_run
    /// use snurr::ProcessBuilder;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: ProcessBuilder<()> = ProcessBuilder::new("examples/counter.bpmn")?;
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
            func_map: Default::default(),
            end_callback: Default::default(),
            intermediate_throw_callback: Default::default(),
            intermediate_catch_callback: Default::default(),
        })
    }

    /// # Task
    ///
    /// All tasks is used in the same way regardless of which icon is used in the BPMN diagram. If a task name
    /// is given then every task with same name will use the same closure. Register a task by **name** or by **id**.
    ///
    /// A name is preferable, since an id can be regenerated in the BPMN tool (if elements are deleted and re-added).
    /// Two or more outgoing sequence flows from a task create a fork of the flow. It is recommended to use a parallel gateway
    /// after the task instead, for the sake of clarity.
    ///
    /// ## Default flow
    ///
    /// Return `Default` if no boundary is used and follow regular flow.
    ///
    /// ```rust no_run
    /// # use snurr::ProcessBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .task("name or id", |input| {
    ///     Default::default()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Boundary flow
    ///
    /// If one or more boundaries exist on a task, then a boundary can be returned. If a name exist it must match.
    ///
    /// ### Boundary with no name
    ///
    /// ```rust no_run
    /// # use snurr::{ProcessBuilder, Symbol};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .task("name or id", |input| {
    ///     Symbol::Error.into()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ### Boundary with name
    ///
    /// ```rust no_run
    /// # use snurr::{ProcessBuilder, Symbol};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .task("name or id", |input| {
    ///     ("Not good", Symbol::Error).into()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn task<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> Task + 'static + Sync + Send,
    {
        self.func_map.insert(
            BpmnType::Task,
            name.into(),
            self.handler.add_callback(Callback::Task(Box::new(func))),
        );
        self
    }

    /// # Exclusive gateway
    ///
    /// An exclusive gateway can select a flow named after the outgoing sequence flow.
    ///
    /// ## One flow
    ///
    /// ```rust no_run
    /// # use snurr::ProcessBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .exclusive("name or id", |input| {
    ///     "YES".into()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Default flow
    ///
    /// ```rust no_run
    /// # use snurr::ProcessBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .exclusive("name or id", |input| {
    ///     Default::default()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn exclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> Exclusive + 'static + Sync + Send,
    {
        self.func_map.insert(
            BpmnType::Exclusive,
            name.into(),
            self.handler
                .add_callback(Callback::Exclusive(Box::new(func))),
        );
        self
    }

    /// # Inclusive gateway
    ///
    /// An inclusive gateway can select one or many flows named after the outgoing sequence flow. A default flow
    /// should always be available in the BPMN diagram. Do not forget to merge the flows using a converging gateway.
    /// Only balanced gateway construction supported.
    ///
    /// ## One flow
    ///
    /// ```rust no_run
    /// # use snurr::ProcessBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .inclusive("name or id", |input| {
    ///     "YES".into()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Many flows
    ///
    /// ```rust no_run
    /// # use snurr::ProcessBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .inclusive("name or id", |input| {
    ///     vec!["YES", "NO"].into()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Default flow
    ///
    /// ```rust no_run
    /// # use snurr::ProcessBuilder;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .inclusive("name or id", |input| {
    ///     Default::default()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn inclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> Inclusive + 'static + Sync + Send,
    {
        self.func_map.insert(
            BpmnType::Inclusive,
            name.into(),
            self.handler
                .add_callback(Callback::Inclusive(Box::new(func))),
        );
        self
    }

    /// # Event-based gateway
    ///
    /// An event-based gateway can select a flow with an intermediate throw event, where the name and symbol must match those of the intermediate catching event. Event-based gateways require at least 2 outputs.
    ///
    /// ## One flow
    ///
    /// ```rust no_run
    /// # use snurr::{ProcessBuilder, Symbol};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .event_based("name or id", |input| {
    ///      ("Message", Symbol::Message).into()
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn event_based<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(&T) -> IntermediateEvent + 'static + Sync + Send,
    {
        self.func_map.insert(
            BpmnType::EventBased,
            name.into(),
            self.handler
                .add_callback(Callback::EventBased(Box::new(func))),
        );
        self
    }

    /// Optionally register an end callback to act on end events. If an error is returned it terminate the process
    /// prematurely and have it return the specified error. Only one can be registered.
    /// ```rust no_run
    /// # use snurr::{ProcessBuilder, Symbol} ;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .end_event(|_input, name, symbol| {      
    ///     match symbol {
    ///         Symbol::Error => println!("act on an error, such as update the model or inform external systems"),
    ///         _ => println!("ignore other end events"),
    ///     }
    ///     Ok(())
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn end_event<F>(mut self, func: F) -> Self
    where
        F: Fn(&T, Option<&str>, Symbol) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
            + 'static
            + Sync
            + Send,
    {
        self.end_callback = self
            .handler
            .add_callback(Callback::EndOrIntermediate(Box::new(func)))
            .into();
        self
    }

    /// Optionally register an intermediate throw callback to act on intermediate throw events. If an error is returned it terminate the process
    /// prematurely and have it return the specified error. Only one can be registered.
    /// ```rust no_run
    /// # use snurr::{ProcessBuilder, Symbol} ;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .intermediate_throw_event(|_input, name, symbol| {      
    ///     match symbol {
    ///         Symbol::Message => println!("act on the message, for example by informing external systems"),
    ///         _ => println!("ignore other throw events"),
    ///     }
    ///     Ok(())
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn intermediate_throw_event<F>(mut self, func: F) -> Self
    where
        F: Fn(&T, Option<&str>, Symbol) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
            + 'static
            + Sync
            + Send,
    {
        self.intermediate_throw_callback = self
            .handler
            .add_callback(Callback::EndOrIntermediate(Box::new(func)))
            .into();
        self
    }

    /// Optionally register an intermediate catch callback to act on intermediate catch events. If an error is returned it terminate the process
    /// prematurely and have it return the specified error. Only one can be registered.
    /// ```rust no_run
    /// # use snurr::{ProcessBuilder, Symbol};
    /// # use std::time::{Duration};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #   ProcessBuilder::<()>::new("dummy.bpmn")?
    /// .intermediate_catch_event(|_input, name, symbol| {
    ///     match (name, symbol) {
    ///         (Some("wait 5 sec"), Symbol::Timer) => std::thread::sleep(Duration::new(5, 0)),
    ///         (Some("wait 1 minute"), Symbol::Timer) => std::thread::sleep(Duration::new(60, 0)),
    ///         _ => println!("ignore other catch events"),
    ///     }
    ///     Ok(())
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn intermediate_catch_event<F>(mut self, func: F) -> Self
    where
        F: Fn(&T, Option<&str>, Symbol) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
            + 'static
            + Sync
            + Send,
    {
        self.intermediate_catch_callback = self
            .handler
            .add_callback(Callback::EndOrIntermediate(Box::new(func)))
            .into();
        self
    }

    /// Install and check that all required functions have been registered. Return runnable process if successful.
    /// If `build` returns an error, it contains the missing functions.
    pub fn build(mut self) -> Result<Process<T>, BuildError> {
        let result = self.diagram.install_and_check(&self.func_map);
        if result.is_empty() {
            Ok(Process {
                diagram: self.diagram,
                handler: self.handler,
                end_callback: self.end_callback,
                intermediate_throw_callback: self.intermediate_throw_callback,
                intermediate_catch_callback: self.intermediate_catch_callback,
            })
        } else {
            Err(BuildError::MissingImplementations(
                result.into_iter().collect::<Vec<_>>().join(", "),
            ))
        }
    }
}

impl<T> FromStr for ProcessBuilder<T> {
    type Err = ParseError;

    /// Create new process builder and initialize it from a BPMN `&str`.
    /// ```no_run
    /// use snurr::ProcessBuilder;
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/counter.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: ProcessBuilder<()> = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            diagram: read_bpmn(quick_xml::Reader::from_str(s))?,
            handler: Default::default(),
            func_map: Default::default(),
            end_callback: Default::default(),
            intermediate_throw_callback: Default::default(),
            intermediate_catch_callback: Default::default(),
        })
    }
}

/// Runnable process created by `ProcessBuilder::build()`
pub struct Process<T>
where
    Self: Sync + Send,
{
    diagram: Diagram,
    handler: Handler<T>,
    end_callback: Option<usize>,
    intermediate_throw_callback: Option<usize>,
    intermediate_catch_callback: Option<usize>,
}

impl<T> Process<T> {
    /// Run the process and return the `T` or an `RuntimeError`.
    /// ```
    /// use snurr::ProcessBuilder;
    /// use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter(AtomicU32);
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///
    ///     // Create process from BPMN file
    ///     let bpmn = ProcessBuilder::<Counter>::new("examples/counter.bpmn")?
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
        self.execute(ExecuteInput::new(
            self.diagram.main_process()?,
            false,
            &data,
        ))?;
        Ok(data)
    }
}

/// Errors that can occur while reading a bpmn file
#[derive(Debug)]
#[non_exhaustive]
pub struct BpmnFileError {
    pub path: Box<std::path::Path>,
    pub source: BpmnFileErrorKind,
}

impl Display for BpmnFileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "error reading `{}`", self.path.display())
    }
}

impl Error for BpmnFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.source {
            BpmnFileErrorKind::ReadFile(e) => Some(e.as_ref()),
            BpmnFileErrorKind::Parse(e) => Some(e),
        }
    }
}

#[derive(Debug)]
pub enum BpmnFileErrorKind {
    ReadFile(Box<dyn std::error::Error + Send + Sync>),
    Parse(ParseError),
}

/// Errors that can occur while parsing bpmn data.
#[derive(Debug)]
#[non_exhaustive]
pub struct ParseError {
    pub source: ParseErrorKind,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "error parsing")
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Debug)]
pub enum ParseErrorKind {
    Bpmn(BpmnError),
    ProcessBuild(String),
    MissingStartEvent,
    NotSupported(String),
    Encoding(Box<dyn std::error::Error + Send + Sync>),
    Xml {
        line: usize,
        column: usize,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::Bpmn(_) => f.write_str("could not create bpmn type"),
            ParseErrorKind::ProcessBuild(s) => write!(f, "{s}"),
            ParseErrorKind::MissingStartEvent => f.write_str("missing start event"),
            ParseErrorKind::NotSupported(s) => write!(f, "`{s}` not supported"),
            ParseErrorKind::Encoding(error) => error.fmt(f),
            ParseErrorKind::Xml { line, column, .. } => {
                write!(f, "xml error on line `{line}` and column `{column}`")
            }
        }
    }
}

impl Error for ParseErrorKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseErrorKind::Bpmn(bpmn_error) => Some(bpmn_error),
            ParseErrorKind::Encoding(error) => error.source(),
            ParseErrorKind::Xml { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl From<ParseErrorKind> for ParseError {
    fn from(value: ParseErrorKind) -> Self {
        Self { source: value }
    }
}

/// Errors that can occur while running the process
#[derive(Debug)]
#[non_exhaustive]
pub enum RuntimeError {
    /// Bpmn diagram design error or user specified wrong value
    Diagram(DiagramError),
    /// Engine problems, should not happen :)
    Engine(String),
    /// User triggered a Panic inside some of the process steps with the attached error
    Panic(Box<dyn std::error::Error + Send + Sync>),
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Diagram(..) => f.write_str("diagram error"),
            RuntimeError::Engine(s) => write!(f, "engine failure `{s}`"),
            RuntimeError::Panic(..) => f.write_str("user triggered panic"),
        }
    }
}

impl Error for RuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RuntimeError::Diagram(diagram_error) => Some(diagram_error),
            RuntimeError::Panic(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

/// Design flaws in bpmn or incorrect use of the diagram
#[derive(Debug)]
pub enum DiagramError {
    MissingOutput(String),
    MissingDefault(String),
    MissingBoundary(String, String),
    MissingIntermediateEvent(String, String),
    MissingIntermediateThrowEventName(String),
    MissingIntermediateCatchEvent(String, String),
    MissingEndEvent,
    NotSupported(String),
    BpmnRequirement(String),
}

impl Display for DiagramError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DiagramError::MissingOutput(s) => {
                write!(f, "{s}")
            }
            DiagramError::MissingDefault(s) => {
                write!(f, "{s} has no default flow")
            }
            DiagramError::MissingBoundary(s1, s2) => {
                write!(f, "could not find {s1} boundary symbol attached to {s2}")
            }
            DiagramError::MissingIntermediateEvent(s1, s2) => {
                write!(
                    f,
                    "event gateway {s1} could not find intermediate catch event {s2}"
                )
            }
            DiagramError::MissingIntermediateThrowEventName(s) => {
                write!(f, "missing intermediate throw event name on {s}")
            }
            DiagramError::MissingIntermediateCatchEvent(s1, s2) => {
                write!(
                    f,
                    "missing intermediate catch event symbol {s1} with name {s2}"
                )
            }
            DiagramError::MissingEndEvent => f.write_str("missing end event"),
            DiagramError::NotSupported(s) => {
                write!(f, "{s} not supported")
            }
            DiagramError::BpmnRequirement(s) => {
                write!(f, "{s}")
            }
        }
    }
}

impl Error for DiagramError {}

impl From<DiagramError> for RuntimeError {
    fn from(source: DiagramError) -> Self {
        RuntimeError::Diagram(source)
    }
}

/// Errors that can occur while trying to make a process runnable
#[derive(Debug)]
pub enum BuildError {
    MissingImplementations(String),
}

impl Display for BuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::MissingImplementations(s) => write!(f, "missing implementations {s}"),
        }
    }
}

impl Error for BuildError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = ProcessBuilder::new("examples/counter.bpmn")?
            .task("Count 1", |_| Default::default())
            .exclusive("equal to 3", |_| Default::default())
            .build()?;
        bpmn.run({})?;
        Ok(())
    }
}
