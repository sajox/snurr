mod engine;
mod parallel;
mod replay;
mod scaffold;

#[cfg(feature = "trace")]
mod trace;

use engine::ExecuteData;
use std::{
    collections::HashMap,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

#[cfg(feature = "trace")]
use trace::{tracer, Trace};

use crate::{
    error::Error,
    model::{Bpmn, BpmnLocal, EventType},
    reader::{read_bpmn_file, read_bpmn_str},
    Eventhandler, Symbol,
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
    activity_ids: HashMap<String, HashMap<Symbol, usize>>,
    catch_events_ids: HashMap<String, HashMap<Symbol, usize>>,
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
        Self::assemble_data(read_bpmn_file(path)?)
    }

    fn assemble_data(
        (definitions_id, data): (String, HashMap<String, Vec<Bpmn>>),
    ) -> Result<Self, Error> {
        // Collect all boundary symbols attached to an activity id
        let mut activity_ids: HashMap<String, HashMap<Symbol, usize>> = HashMap::new();

        // Collect all IntermediateCatchEvents
        let mut catch_events_ids: HashMap<String, HashMap<Symbol, usize>> = HashMap::new();

        data.values().for_each(|process: &Vec<Bpmn>| {
            process.iter().for_each(|bpmn| {
                if let Bpmn::Event {
                    id,
                    event: EventType::Boundary,
                    symbol: Some(symbol),
                    attached_to_ref: Some(BpmnLocal(bref, _)),
                    ..
                } = bpmn
                {
                    let entry = activity_ids.entry(bref.into()).or_default();
                    entry.insert(symbol.clone(), *id.local());
                }

                if let Bpmn::Event {
                    id,
                    event: EventType::IntermediateCatch,
                    symbol: Some(symbol),
                    name: Some(name),
                    ..
                } = bpmn
                {
                    let entry = catch_events_ids.entry(name.into()).or_default();
                    entry.insert(symbol.clone(), *id.local());
                }
            });
        });

        Ok(Self {
            data,
            definitions_id,
            activity_ids,
            catch_events_ids,
        })
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
                let (process_id, process_data) = self
                    .data
                    .get_key_value(id)
                    .ok_or_else(|| Error::MissingProcessData(id.into()))?;

                self.execute(
                    vec![&0],
                    &ExecuteData::new(
                        process_id,
                        process_data,
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
        Self::assemble_data(read_bpmn_str(s)?)
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
