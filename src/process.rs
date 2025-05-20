mod diagram;
mod engine;
pub mod handler;
mod reader;
mod scaffold;

use crate::{IntermediateEvent, With, error::Error, model::Bpmn};
use diagram::Diagram;
use engine::ExecuteData;
use handler::{Data, Handler, TaskResult};
use std::{
    marker::PhantomData,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

/// Process that contains information from the BPMN file
pub struct Process<S, T> {
    diagram: Diagram,
    handler: Handler<T>,
    _marker: PhantomData<S>,
}

/// Process Build state
pub struct Build;

/// Process Run state
pub struct Run;

impl<T> Process<Build, T> {
    /// Create new process and initialize it from the BPMN file path.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<_, ()> = Process::new("examples/example.bpmn")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            diagram: reader::read_bpmn(quick_xml::Reader::from_file(path)?)?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }

    pub fn task<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        self.handler.add_task(name, func);
        self
    }

    pub fn exclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync,
    {
        self.handler.add_exclusive(name, func);
        self
    }

    pub fn inclusive<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        self.handler.add_inclusive(name, func);
        self
    }

    pub fn event_based<F>(mut self, name: impl Into<String>, func: F) -> Self
    where
        F: Fn(Data<T>) -> IntermediateEvent + 'static + Sync,
    {
        self.handler.add_event_based(name, func);
        self
    }

    pub fn build(mut self) -> Result<Process<Run, T>, Error> {
        let result = self
            .diagram
            .install_and_check(self.handler.take_handler_map());
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

impl<T> FromStr for Process<Build, T> {
    type Err = Error;

    /// Create new process and initialize it from a BPMN `&str`.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/example.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<_, ()> = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            diagram: reader::read_bpmn(quick_xml::Reader::from_str(s))?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl<T> Process<Run, T> {
    /// Run the process and return the `T` or an `Error`.
    /// ```
    /// use snurr::Process;
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     pretty_env_logger::init();
    ///
    ///     // Create process from BPMN file
    ///     let bpmn = Process::<_, Counter>::new("examples/example.bpmn")?
    ///         .task("Count 1", |input| {
    ///             input.lock().unwrap().count += 1;
    ///             None
    ///         })
    ///         .exclusive("equal to 3", |input| {
    ///             let result = if input.lock().unwrap().count == 3 {
    ///                 "YES"
    ///             } else {
    ///                 "NO"
    ///             };
    ///             result.into()
    ///         })
    ///         .build()?;
    ///
    ///     // Run the process with input data
    ///     let counter = bpmn.run(Counter::default())?;
    ///
    ///     // Print the result.
    ///     println!("Count: {}", counter.count);
    ///     Ok(())
    /// }
    /// ```
    pub fn run(&self, data: T) -> Result<T, Error>
    where
        T: Send,
    {
        let data = Arc::new(Mutex::new(data));

        // Run every process specified in the diagram
        for bpmn in self
            .diagram
            .get_processes()
            .ok_or(Error::MissingDefinitionsId)?
            .iter()
        {
            if let Bpmn::Process { id, .. } = bpmn {
                let process_data = self
                    .diagram
                    .get_process(*id.local())
                    .ok_or_else(|| Error::MissingProcessData(id.bpmn().into()))?;
                self.execute(&ExecuteData::new(process_data, id, Arc::clone(&data)))?;
            }
        }

        Arc::into_inner(data)
            .ok_or(Error::NoProcessResult)?
            .into_inner()
            .map_err(|_| Error::NoProcessResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?
            .task("Count 1", |_| None)
            .exclusive("equal to 3", |_| None)
            .build()?;
        bpmn.run({})?;
        Ok(())
    }
}
