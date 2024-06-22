mod engine;
mod parallel;
mod replay;
mod scaffold;
mod trace;

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};
use trace::{tracer, Trace};

use crate::{
    error::Error,
    model::{Bpmn, EventType},
    reader::read_bpmn_file,
    Eventhandler, Symbol,
};

pub(crate) type ExecuteResult<'a> = Result<Vec<&'a str>, Error>;

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
    data: HashMap<String, HashMap<String, Bpmn>>,
    definitions_id: String,
    activity_ids: HashMap<String, HashMap<Symbol, String>>,
    catch_events_ids: HashMap<String, HashMap<Symbol, String>>,
}

impl Process {
    /// Create new process and initialize it from the BPMN file path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let (definitions_id, mut data) = read_bpmn_file(path)?;

        // Collect all referencing output names
        let mut gateway_ids: HashMap<String, HashMap<String, String>> = HashMap::new();

        // Collect all boundary symbols attached to an activity id
        let mut activity_ids: HashMap<String, HashMap<Symbol, String>> = HashMap::new();

        // Collect all IntermediateCatchEvents
        let mut catch_events_ids: HashMap<String, HashMap<Symbol, String>> = HashMap::new();

        data.values().for_each(|process: &HashMap<String, Bpmn>| {
            process.values().for_each(|bpmn| {
                if let Bpmn::SequenceFlow {
                    id,
                    name: Some(name),
                    source_ref,
                    ..
                } = bpmn
                {
                    if let Some(Bpmn::Gateway { .. }) = process.get(source_ref) {
                        let entry = gateway_ids.entry(source_ref.into()).or_default();
                        entry.insert(name.into(), id.into());
                    }
                }

                if let Bpmn::Event {
                    event: EventType::Boundary,
                    symbol: Some(symbol),
                    id,
                    attached_to_ref: Some(attached_to_ref),
                    ..
                } = bpmn
                {
                    let entry = activity_ids.entry(attached_to_ref.into()).or_default();
                    entry.insert(symbol.clone(), id.into());
                }

                if let Bpmn::Event {
                    event: EventType::IntermediateCatch,
                    symbol: Some(symbol),
                    id,
                    name: Some(name),
                    ..
                } = bpmn
                {
                    let entry = catch_events_ids.entry(name.into()).or_default();
                    entry.insert(symbol.clone(), id.into());
                }
            });
        });

        // Update gateway outputs with name
        data.values_mut().for_each(|process| {
            process.values_mut().for_each(|bpmn| {
                if let Bpmn::Gateway { id, outputs, .. } = bpmn {
                    if let Some(map) = gateway_ids.get(id) {
                        for (name, id) in map.iter() {
                            outputs.register_name(id, name);
                        }
                    }
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
    pub fn run<T>(&self, handler: &Eventhandler<T>, data: T) -> Result<ProcessResult<T>, Error>
    where
        T: Send + std::fmt::Debug,
    {
        let data = Arc::new(Mutex::new(data));
        let trace: Trace<(&str, String)> = tracer();

        // Run every process specified in the diagram
        for (_, bpmn) in self
            .data
            .get(&self.definitions_id)
            .ok_or(Error::MissingDefinitions)?
            .iter()
        {
            if let Bpmn::Process {
                id,
                start_id: Some(start_id),
                ..
            } = bpmn
            {
                self.execute(
                    vec![start_id],
                    self.data.get_key_value(id).ok_or(Error::MissingProcess)?,
                    handler,
                    Arc::clone(&data),
                    trace.sender(),
                )?;
            }
        }

        Ok(ProcessResult {
            result: Arc::into_inner(data)
                .ok_or(Error::NoResult)?
                .into_inner()
                .map_err(|_| Error::NoResult)?,
            trace: trace.finish(),
        })
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
