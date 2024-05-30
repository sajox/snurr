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
    model::{Bpmn, BpmnType},
    reader::read_bpmn_file,
    scaffold::Scaffold,
    Data, Eventhandler, Symbol,
};

type ExecuteResult<'a> = Result<&'a String, Error>;

/// Process result from a process run.
#[derive(Debug)]
pub struct ProcessResult<T> {
    /// Result produced by the task flow.
    pub result: T,

    /// Trace from the process run
    pub trace: Vec<(BpmnType, String)>,
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
                    event: BpmnType::BoundaryEvent,
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
                    event: BpmnType::IntermediateCatchEvent,
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
                        aktivity: BpmnType::Task,
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
                        gateway: BpmnType::ExclusiveGateway | BpmnType::InclusiveGateway,
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

    fn name_lookup(&self, gateway_id: &str, name: &str) -> Option<&String> {
        self.gateway_ids
            .get(gateway_id)
            .and_then(|map| map.get(name))
    }

    fn boundary_lookup(&self, activity_id: &str, symbol: &Symbol) -> Option<&String> {
        self.activity_ids
            .get(activity_id)
            .and_then(|map| map.get(symbol))
    }

    fn catch_event_lookup(&self, throw_event_name: &str, symbol: &Symbol) -> Option<&String> {
        self.catch_events_ids
            .get(throw_event_name)
            .and_then(|map| map.get(symbol))
    }

    /// Replay a trace from a process run. It will be sequential.
    pub fn replay_trace<T>(handler: &Eventhandler<T>, data: T, trace: &[(BpmnType, String)]) -> T
    where
        T: std::fmt::Debug,
    {
        let data = Arc::new(Mutex::new(data));
        for (ty, id) in trace {
            match ty {
                BpmnType::Task | BpmnType::SubProcess => {
                    let _ = handler.run_task(id, Arc::clone(&data));
                }
                BpmnType::ExclusiveGateway
                | BpmnType::InclusiveGateway
                | BpmnType::ParallelGateway => {
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
        T: Send + Sync + std::fmt::Debug,
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
                    self.data.get(id).ok_or(Error::MissingProcess)?,
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
        start_id: &'a String,
        process_data: &'a HashMap<String, Bpmn>,
        handler: &Eventhandler<T>,
        data: Data<T>,
        sender: Sender<(BpmnType, String)>,
    ) -> ExecuteResult<'a>
    where
        T: Send + Sync,
    {
        let mut next_id = start_id;
        loop {
            let bpmn = process_data
                .get(next_id)
                .ok_or_else(|| Error::MissingId(next_id.into()))?;

            // We trace by name if exist. Id otherwise.
            let _ = sender.send((
                BpmnType::from(bpmn),
                bpmn.name()
                    .or_else(|| bpmn.id())
                    .cloned()
                    .unwrap_or_default(),
            ));

            next_id = match bpmn {
                Bpmn::Process { start_id, .. } => {
                    start_id.as_ref().ok_or(Error::MissingProcessStart)?
                }
                Bpmn::Event {
                    event,
                    symbol,
                    id,
                    name,
                    output,
                    ..
                } => {
                    info!("{}: {}", event, name.as_ref().unwrap_or(id));
                    let output = output
                        .as_ref()
                        .ok_or_else(|| Error::MissingOutput(event.to_string()));
                    match event {
                        BpmnType::StartEvent
                        | BpmnType::IntermediateCatchEvent
                        | BpmnType::BoundaryEvent => output?,
                        BpmnType::IntermediateThrowEvent => {
                            // If no symbol is set then just follow output.
                            if symbol.as_ref().is_none() {
                                output?
                            } else {
                                match name.as_ref().zip(symbol.as_ref()) {
                                    Some((name, symbol @ Symbol::Link)) => {
                                        self.catch_event_lookup(name, symbol).ok_or_else(|| {
                                            Error::MissingIntermediateCatchEvent(name.into())
                                        })?
                                    }
                                    Some((_, _)) => output?,
                                    None => {
                                        Err(Error::MissingNameIntermediateThrowEvent(id.into()))?
                                    }
                                }
                            }
                        }
                        BpmnType::EndEvent => break,
                        _ => return Err(Error::BadEventType),
                    }
                }
                Bpmn::Activity {
                    aktivity,
                    id,
                    name,
                    output,
                    start_id,
                } => {
                    let name_or_id = name.as_ref().unwrap_or(id);
                    info!("{}: {}", aktivity, name_or_id);
                    match aktivity {
                        BpmnType::Task => {
                            let response = handler.run_task(name_or_id, Arc::clone(&data));
                            if let Err(symbol) = response {
                                self.boundary_lookup(id, &symbol)
                                    .ok_or_else(|| Error::MissingBoundary(name_or_id.into()))?
                            } else {
                                output
                                    .as_ref()
                                    .ok_or_else(|| Error::MissingOutput(aktivity.to_string()))?
                            }
                        }
                        BpmnType::SubProcess => {
                            let sub_process_data =
                                self.data.get(id).ok_or(Error::MissingSubProcess)?;
                            let response_id = self.execute(
                                start_id.as_ref().ok_or(Error::MissingProcessStart)?,
                                sub_process_data,
                                handler,
                                Arc::clone(&data),
                                sender.clone(),
                            )?;

                            if let Some(Bpmn::Event {
                                event: BpmnType::EndEvent,
                                symbol: Some(symbol),
                                ..
                            }) = sub_process_data.get(response_id)
                            {
                                self.boundary_lookup(id, symbol)
                                    .ok_or_else(|| Error::MissingBoundary(name_or_id.into()))?
                            } else {
                                output
                                    .as_ref()
                                    .ok_or_else(|| Error::MissingOutput(aktivity.to_string()))?
                            }
                        }
                        _ => return Err(Error::BadActivityType),
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
                    match gateway {
                        BpmnType::ExclusiveGateway => {
                            if outputs.len() > 1 {
                                // Default to first outgoing if function is not set.
                                let response = handler.run_gateway(name_or_id, Arc::clone(&data));
                                let response = response
                                    .first()
                                    .copied()
                                    .or(default.as_deref())
                                    .or_else(|| outputs.first().map(|x| x.as_str()))
                                    .ok_or_else(|| Error::MissingId(id.into()))?;

                                // look up name to id or just use answer
                                self.name_lookup(id, response)
                                    .or_else(|| {
                                        outputs.iter().find(|&outgoing| outgoing == response)
                                    })
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()))?
                            } else {
                                outputs
                                    .first()
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()))?
                            }
                        }
                        BpmnType::InclusiveGateway => {
                            // Converging gateway. Synchronize
                            if outputs.len() <= 1 {
                                return outputs
                                    .first()
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()));
                            }

                            // Diverging gateway
                            let mut response = handler.run_gateway(name_or_id, Arc::clone(&data));
                            // If empty. Add default or first output.
                            if response.is_empty() {
                                if let Some(resp) = default.as_ref().or_else(|| outputs.first()) {
                                    response.push(resp);
                                }
                            }

                            // Run all chosen paths
                            let (oks, mut errors): (Vec<_>, Vec<_>) = response
                                .iter()
                                .filter_map(|response| {
                                    self.name_lookup(id, response).or_else(|| {
                                        outputs.iter().find(|&outgoing| outgoing == response)
                                    })
                                })
                                .map(|outgoing| {
                                    self.execute(
                                        outgoing,
                                        process_data,
                                        handler,
                                        Arc::clone(&data),
                                        sender.clone(),
                                    )
                                })
                                .partition(Result::is_ok);

                            if !errors.is_empty() {
                                if let Some(result) = errors.pop() {
                                    return result;
                                }
                            }

                            oks.into_iter()
                                .filter_map(Result::ok)
                                .next()
                                .ok_or_else(|| {
                                    Error::BadGatewayOutput(format!(
                                        "No output with name: {}",
                                        response.join(", ")
                                    ))
                                })?
                        }
                        BpmnType::ParallelGateway => {
                            // Converging gateway. Synchronize
                            if outputs.len() <= 1 {
                                return outputs
                                    .first()
                                    .ok_or_else(|| Error::MissingOutput(gateway.to_string()));
                            }

                            // Diverging gateway
                            let (oks, mut errors): (Vec<_>, Vec<_>) = thread::scope(|s| {
                                //Start everything first
                                let children: Vec<_> = outputs
                                    .iter()
                                    .map(|outgoing| {
                                        s.spawn(|| {
                                            self.execute(
                                                outgoing,
                                                process_data,
                                                handler,
                                                Arc::clone(&data),
                                                sender.clone(),
                                            )
                                        })
                                    })
                                    .collect();

                                // Collect results
                                children
                                    .into_iter()
                                    .filter_map(|handle| handle.join().ok())
                                    .partition(Result::is_ok)
                            });

                            if !errors.is_empty() {
                                if let Some(result) = errors.pop() {
                                    return result;
                                }
                            }

                            oks.into_iter()
                                .filter_map(Result::ok)
                                .next()
                                .ok_or(Error::NoGatewayOutput)?
                        }
                        _ => return Err(Error::BadGatewayType),
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
                bpmn => return Err(Error::MissingBpmnType(BpmnType::from(bpmn).to_string())),
            }
        }
        // Return EndEvent Id
        Ok(next_id)
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
