mod parallel;
mod replay;
mod scaffold;
mod trace;

use parallel::parallelize_helper;

use log::info;
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    sync::{mpsc::Sender, Arc, Mutex},
};
use trace::{tracer, Trace};

use crate::{
    error::Error,
    model::{ActivityType, Bpmn, EventType, GatewayType},
    reader::read_bpmn_file,
    Data, Eventhandler, Symbol,
};

type ExecuteResult<'a> = Result<Vec<&'a str>, Error>;

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

    fn boundary_lookup(&self, activity_id: &str, symbol: &Symbol) -> Option<&String> {
        self.activity_ids
            .get(activity_id)
            .and_then(|map| map.get(symbol))
    }

    fn catch_event_lookup(
        &self,
        throw_event_name: &str,
        symbol: &Symbol,
    ) -> Result<&String, Error> {
        self.catch_events_ids
            .get(throw_event_name)
            .and_then(|map| map.get(symbol))
            .ok_or_else(|| Error::MissingIntermediateCatchEvent(throw_event_name.into()))
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

    fn execute<'a, T>(
        &'a self,
        ids: Vec<&'a str>,
        process @ (process_id, process_data): (&String, &'a HashMap<String, Bpmn>),
        handler: &Eventhandler<T>,
        data: Data<T>,
        sender: Sender<(&'static str, String)>,
    ) -> ExecuteResult<'a>
    where
        T: Send,
    {
        let recursion = |outputs: Vec<&'a str>| {
            self.execute(outputs, process, handler, Arc::clone(&data), sender.clone())
        };

        let mut results: Vec<&str> = vec![];
        let mut queue = VecDeque::from(ids);
        while let Some(next_id) = queue.pop_front() {
            queue.push_back(
                match process_data
                    .get(next_id)
                    .ok_or_else(|| Error::MissingId(next_id.to_string()))?
                {
                    Bpmn::Process { start_id, .. } => {
                        start_id.as_ref().ok_or(Error::MissingProcessStart)?
                    }
                    Bpmn::Event {
                        event,
                        symbol,
                        id,
                        name,
                        outputs,
                        ..
                    } => {
                        info!("{}: {}", event, name.as_ref().unwrap_or(id));
                        match event {
                            EventType::Start
                            | EventType::IntermediateCatch
                            | EventType::Boundary => {
                                parallelize_helper!(outputs, recursion)
                            }
                            EventType::IntermediateThrow => {
                                match (name.as_ref(), symbol.as_ref()) {
                                    (Some(name), Some(symbol @ Symbol::Link)) => {
                                        self.catch_event_lookup(name, symbol)?
                                    }
                                    // Follow outputs for other throw events
                                    (Some(_), _) => {
                                        parallelize_helper!(outputs, recursion)
                                    }
                                    _ => Err(Error::MissingNameIntermediateThrowEvent(id.into()))?,
                                }
                            }
                            EventType::End => {
                                if let Some(boundary_id) = symbol.as_ref().and_then(|symbol| {
                                    self.boundary_lookup(process_id, symbol).map(String::as_str)
                                }) {
                                    results.push(boundary_id);
                                }
                                continue;
                            }
                        }
                    }
                    Bpmn::Activity {
                        aktivity,
                        id,
                        name,
                        outputs,
                        start_id,
                        ..
                    } => {
                        let name_or_id = name.as_ref().unwrap_or(id);
                        info!("{}: {}", aktivity, name_or_id);
                        match aktivity {
                            ActivityType::Task => {
                                let _ = sender.send((replay::TASK, name_or_id.to_owned()));
                                match handler.run_task(name_or_id, Arc::clone(&data)) {
                                    Ok(_) => parallelize_helper!(outputs, recursion),
                                    Err(symbol) => self
                                        .boundary_lookup(id, &symbol)
                                        .ok_or_else(|| Error::MissingBoundary(name_or_id.into()))?,
                                }
                            }
                            ActivityType::SubProcess => {
                                let sub_process = self
                                    .data
                                    .get_key_value(id)
                                    .ok_or(Error::MissingSubProcess)?;

                                match self
                                    .execute(
                                        vec![start_id
                                            .as_ref()
                                            .ok_or(Error::MissingProcessStart)?],
                                        sub_process,
                                        handler,
                                        Arc::clone(&data),
                                        sender.clone(),
                                    )?
                                    .as_slice()
                                {
                                    // Boundary id returned
                                    &[id, ..] => id,
                                    // Continue from subprocess
                                    _ => parallelize_helper!(outputs, recursion),
                                }
                            }
                        }
                    }
                    // Join
                    Bpmn::Gateway {
                        gateway,
                        id,
                        name,
                        outputs,
                        ..
                    } if outputs.len() <= 1 => {
                        let name_or_id = name.as_ref().unwrap_or(id);
                        info!("{}: {}", gateway, name_or_id);
                        let _ = sender.send((replay::GATEWAY, name_or_id.to_owned()));
                        match gateway {
                            GatewayType::Exclusive => outputs
                                .first()
                                .ok_or_else(|| Error::MissingOutput(gateway.to_string()))?,
                            GatewayType::Inclusive | GatewayType::Parallel => {
                                results.push(
                                    outputs
                                        .first()
                                        .ok_or_else(|| Error::MissingOutput(gateway.to_string()))?,
                                );
                                continue;
                            }
                        }
                    }
                    // Fork
                    Bpmn::Gateway {
                        gateway,
                        id,
                        name,
                        default,
                        outputs,
                        ..
                    } if outputs.len() > 1 => {
                        let name_or_id = name.as_ref().unwrap_or(id);
                        info!("{}: {}", gateway, name_or_id);
                        let _ = sender.send((replay::GATEWAY, name_or_id.to_owned()));
                        match gateway {
                            GatewayType::Exclusive => {
                                // Default to first outgoing if function is not set.
                                let responses = handler.run_gateway(name_or_id, Arc::clone(&data));
                                responses
                                    .first()
                                    .map(|response| {
                                        outputs.name_to_id(response).unwrap_or(response)
                                    })
                                    .or(default.as_deref())
                                    .or_else(|| outputs.first().map(|x| x.as_str()))
                                    .ok_or_else(|| Error::MissingId(id.into()))?
                            }
                            GatewayType::Inclusive => {
                                let mut responses =
                                    handler.run_gateway(name_or_id, Arc::clone(&data));
                                // If empty. Add default or first output.
                                if responses.is_empty() {
                                    if let Some(resp) = default.as_ref().or_else(|| outputs.first())
                                    {
                                        responses.push(resp);
                                    }
                                }

                                // Run all chosen paths
                                let responses = responses
                                    .iter()
                                    .map(|response| {
                                        outputs.name_to_id(response).unwrap_or(response)
                                    })
                                    .collect();

                                match recursion(responses)?.as_slice() {
                                    &[id, ..] => id,
                                    _ => continue,
                                }
                            }
                            GatewayType::Parallel => parallelize_helper!(outputs, recursion),
                        }
                    }
                    Bpmn::SequenceFlow {
                        id,
                        name,
                        target_ref,
                        ..
                    } => {
                        info!("SequenceFlow: {}", name.as_ref().unwrap_or(id));
                        target_ref
                    }
                    _ => return Err(Error::MissingBpmnType("Type not handled.".into())),
                },
            );
        }
        Ok(results)
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
