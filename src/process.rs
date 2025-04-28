mod engine;
mod reader;
mod replay;
mod scaffold;

#[cfg(feature = "trace")]
mod trace;

use engine::ExecuteData;
use std::{
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

#[cfg(feature = "trace")]
use trace::{Trace, tracer};

use crate::{
    Eventhandler,
    error::Error,
    model::{Bpmn, HashMap},
};

/// Process result from a process run.
#[derive(Debug)]
pub struct ProcessResult<T> {
    /// Result produced by the task flow.
    pub result: T,

    /// Trace from the process run
    pub trace: Vec<(&'static str, String)>,
}

/// Process that contains information from the BPMN file
#[derive(Debug)]
pub struct Process {
    data: HashMap<String, Vec<Bpmn>>,
    definitions_id: String,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
}

impl Process {
    /// Create new process and initialize it from the BPMN file path.
    /// ```
    /// use snurr::Process;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn = Process::new("examples/example.bpmn")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        reader::read_bpmn(quick_xml::Reader::from_file(path)?)
    }

    /// Run the process and return the `ProcessResult` or an `Error`.
    /// ```
    /// use snurr::{Process, Eventhandler};
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn = Process::new("examples/example.bpmn")?;
    ///     let handler: Eventhandler<Counter> = Eventhandler::default();
    ///     // Register Task and Gateways to handler here...
    ///     let pr = bpmn.run(&handler, Counter::default())?;
    ///     Ok(())
    /// }
    /// ```
    pub fn run<T>(&self, handler: &Eventhandler<T>, data: T) -> Result<ProcessResult<T>, Error>
    where
        T: Send,
    {
        let data = Arc::new(Mutex::new(data));

        #[cfg(feature = "trace")]
        let trace: Trace<(&str, String)> = tracer();

        // Run every process specified in the diagram
        for bpmn in self
            .data
            .get(&self.definitions_id)
            .ok_or(Error::MissingDefinitionsId)?
            .iter()
        {
            if let Bpmn::Process { id, .. } = bpmn {
                let process_data = self
                    .data
                    .get(id)
                    .ok_or_else(|| Error::MissingProcessData(id.into()))?;

                self.execute(
                    vec![&0],
                    &ExecuteData::new(
                        process_data,
                        id,
                        handler,
                        Arc::clone(&data),
                        #[cfg(feature = "trace")]
                        trace.sender(),
                    ),
                )?;
            }
        }

        Ok(ProcessResult {
            result: Arc::into_inner(data)
                .ok_or(Error::NoProcessResult)?
                .into_inner()
                .map_err(|_| Error::NoProcessResult)?,
            #[cfg(feature = "trace")]
            trace: trace.finish(),
            #[cfg(not(feature = "trace"))]
            trace: vec![],
        })
    }
}

impl FromStr for Process {
    type Err = Error;

    /// Create new process and initialize it from a BPMN `&str`.
    /// ```
    /// use snurr::Process;
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/example.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        reader::read_bpmn(quick_xml::Reader::from_str(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?;
        let handler: Eventhandler<_> = Eventhandler::default();
        bpmn.run(&handler, {})?;
        Ok(())
    }
}
