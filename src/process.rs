mod parallel;

use parallel::parallelize_helper;

use log::info;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    },
    thread,
};

use crate::{
    error::Error,
    model::{ActivityType, Bpmn, EventType, GatewayType},
    reader::read_bpmn_file,
    scaffold::Scaffold,
    Data, Eventhandler, Symbol,
};

type ExecuteResult<'a> = Result<Option<&'a str>, Error>;

const GATEWAY: &str = "Gateway";
const TASK: &str = "Task";

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
    gateway_ids: HashMap<String, HashMap<String, String>>,
    activity_ids: HashMap<String, HashMap<Symbol, String>>,
    catch_events_ids: HashMap<String, HashMap<Symbol, String>>,
}

impl Process {
    /// Create new process and initialize it from the BPMN file path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let (definitions_id, data) = read_bpmn_file(path)?;

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

        Ok(Self {
            data,
            definitions_id,
            gateway_ids,
            activity_ids,
            catch_events_ids,
        })
    }

    /// Generate code from all the task and gateways to the given file path.
    /// No file is allowed to exist at the target location.
    pub fn scaffold(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut scaffold = Scaffold::default();
        self.data
            .values()
            .for_each(|process: &HashMap<String, Bpmn>| {
                process.values().for_each(|bpmn| {
                    if let Bpmn::Activity {
                        aktivity: ActivityType::Task,
                        id,
                        ..
                    } = bpmn
                    {
                        let symbols = if let Some(map) = self.activity_ids.get(id) {
                            map.keys().collect::<Vec<&Symbol>>()
                        } else {
                            Vec::new()
                        };
                        scaffold.add_task(bpmn, symbols);
                    }

                    if let Bpmn::Gateway {
                        gateway: GatewayType::Exclusive | GatewayType::Inclusive,
                        id,
                        outputs,
                        ..
                    } = bpmn
                    {
                        if outputs.len() > 1 {
                            let names = if let Some(map) = self.gateway_ids.get(id) {
                                let mut names = map.keys().collect::<Vec<&String>>();
                                names.sort();
                                names
                            } else {
                                Vec::new()
                            };
                            scaffold.add_gateway(bpmn, names);
                        }
                    }
                });
            });

        scaffold.create(path)
    }

    fn name_lookup(&self, gateway_id: &str, name: &str) -> Option<&str> {
        self.gateway_ids
            .get(gateway_id)
            .and_then(|map| map.get(name))
            .map(|s| s.as_str())
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

    /// Replay a trace from a process run. It will be sequential. Only Tasks and gateways is traced that might mutate data.
    pub fn replay_trace<T>(
        handler: &Eventhandler<T>,
        data: T,
        trace: &[(impl AsRef<str>, impl AsRef<str>)],
    ) -> T
    where
        T: std::fmt::Debug,
    {
        let data = Arc::new(Mutex::new(data));
        for (ty, id) in trace.iter().map(|(ty, id)| (ty.as_ref(), id.as_ref())) {
            match ty {
                TASK => {
                    let _ = handler.run_task(id, Arc::clone(&data));
                }
                GATEWAY => {
                    let _ = handler.run_gateway(id, Arc::clone(&data));
                }
                _ => {}
            }
        }
        Arc::try_unwrap(data).unwrap().into_inner().unwrap()
    }

    /// Run the process and return the `ProcessResult` or an `Error`.
    pub fn run<T>(&self, handler: &Eventhandler<T>, data: T) -> Result<ProcessResult<T>, Error>
    where
        T: Send + std::fmt::Debug,
    {
        let data = Arc::new(Mutex::new(data));
        let (sender, receiver) = channel();

        // Collect the name or id for the path taken
        let recv_handle = thread::spawn(move || {
            let mut trace = vec![];
            while let Ok(value) = receiver.recv() {
                trace.push(value);
            }
            trace
        });

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
                    start_id,
                    self.data.get_key_value(id).ok_or(Error::MissingProcess)?,
                    handler,
                    Arc::clone(&data),
                    sender.clone(),
                )?;
            }
        }

        // We have one left because we clone() every sender in the loop.
        drop(sender);

        // When sender die, the recv handle terminates.
        let trace = recv_handle.join().expect("oops! the child thread panicked");

        Ok(ProcessResult {
            result: Arc::into_inner(data)
                .ok_or(Error::NoResult)?
                .into_inner()
                .map_err(|_| Error::NoResult)?,
            trace,
        })
    }

    fn execute<'a, T>(
        &'a self,
        mut next_id: &'a str,
        process @ (process_id, process_data): (&String, &'a HashMap<String, Bpmn>),
        handler: &Eventhandler<T>,
        data: Data<T>,
        sender: Sender<(&'static str, String)>,
    ) -> ExecuteResult<'a>
    where
        T: Send,
    {
        let recursion = |output: &'a str| {
            self.execute(output, process, handler, Arc::clone(&data), sender.clone())
        };

        while let Some(bpmn) = process_data.get(next_id) {
            next_id = match bpmn {
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
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            parallelize_helper!(outputs, id, recursion)
                        }
                        EventType::IntermediateThrow => {
                            // If no symbol is set then just follow output.
                            if symbol.as_ref().is_none() {
                                parallelize_helper!(outputs, id, recursion)
                            } else {
                                match name.as_ref().zip(symbol.as_ref()) {
                                    Some((name, symbol @ Symbol::Link)) => {
                                        self.catch_event_lookup(name, symbol)?
                                    }
                                    Some((_, _)) => {
                                        parallelize_helper!(outputs, id, recursion)
                                    }
                                    None => {
                                        Err(Error::MissingNameIntermediateThrowEvent(id.into()))?
                                    }
                                }
                            }
                        }
                        EventType::End => {
                            if let Some(symbol) = symbol {
                                return Ok(self
                                    .boundary_lookup(process_id, symbol)
                                    .map(String::as_str));
                            }
                            return Ok(None);
                        }
                    }
                }
                Bpmn::Activity {
                    aktivity,
                    id,
                    name,
                    outputs,
                    start_id,
                } => {
                    let name_or_id = name.as_ref().unwrap_or(id);
                    info!("{}: {}", aktivity, name_or_id);
                    match aktivity {
                        ActivityType::Task => {
                            let _ = sender.send((TASK, name_or_id.to_owned()));
                            let response = handler.run_task(name_or_id, Arc::clone(&data));
                            if let Err(symbol) = response {
                                self.boundary_lookup(id, &symbol)
                                    .ok_or_else(|| Error::MissingBoundary(name_or_id.into()))?
                            } else {
                                parallelize_helper!(outputs, id, recursion)
                            }
                        }
                        ActivityType::SubProcess => {
                            let sub_process = self
                                .data
                                .get_key_value(id)
                                .ok_or(Error::MissingSubProcess)?;
                            let response_id = self.execute(
                                start_id.as_ref().ok_or(Error::MissingProcessStart)?,
                                sub_process,
                                handler,
                                Arc::clone(&data),
                                sender.clone(),
                            )?;

                            if let Some(response_id) = response_id {
                                response_id
                            } else {
                                parallelize_helper!(outputs, id, recursion)
                            }
                        }
                    }
                }
                Bpmn::Gateway {
                    gateway,
                    id,
                    name,
                    default,
                    outputs,
                } => {
                    let name_or_id = name.as_ref().unwrap_or(id);
                    info!("{}: {}", gateway, name_or_id);
                    let _ = sender.send((GATEWAY, name_or_id.to_owned()));
                    match gateway {
                        GatewayType::Exclusive => {
                            if outputs.len() > 1 {
                                // Default to first outgoing if function is not set.
                                let responses = handler.run_gateway(name_or_id, Arc::clone(&data));
                                responses
                                    .first()
                                    .map(|response| {
                                        self.name_lookup(id, response).unwrap_or(response)
                                    })
                                    .or(default.as_deref())
                                    .or_else(|| outputs.first().map(|x| x.as_str()))
                                    .ok_or_else(|| Error::MissingId(id.into()))?
                            } else {
                                outputs
                                    .first()
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()))?
                            }
                        }
                        GatewayType::Inclusive => {
                            // Converging gateway. Synchronize
                            if outputs.len() <= 1 {
                                return outputs
                                    .first()
                                    .map(String::as_str)
                                    .map(Some)
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()));
                            }

                            // Diverging gateway
                            let mut responses = handler.run_gateway(name_or_id, Arc::clone(&data));
                            // If empty. Add default or first output.
                            if responses.is_empty() {
                                if let Some(resp) = default.as_ref().or_else(|| outputs.first()) {
                                    responses.push(resp);
                                }
                            }

                            // Run all chosen paths
                            let (oks, mut errors): (Vec<_>, Vec<_>) = responses
                                .iter()
                                .map(|response| self.name_lookup(id, response).unwrap_or(response))
                                .map(recursion)
                                .partition(Result::is_ok);

                            if let Some(result) = errors.pop() {
                                return result;
                            }

                            match oks
                                .into_iter()
                                .filter_map(Result::ok)
                                .find(Option::is_some)
                                .flatten()
                            {
                                Some(id) => id,
                                None => return Ok(None),
                            }
                        }
                        GatewayType::Parallel => {
                            // Converging gateway. Synchronize
                            if outputs.len() <= 1 {
                                return outputs
                                    .first()
                                    .map(String::as_str)
                                    .map(Some)
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()));
                            }
                            parallelize_helper!(outputs, id, recursion)
                        }
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
            }
        }
        Err(Error::MissingId(next_id.into()))
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
