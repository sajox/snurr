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
    model::{Bpmn, BpmnType, Eventhandler},
    reader::read_bpmn_file,
    Data, Symbol,
};

const STARTEVENT_PREFIX: &str = "start";
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
    data: HashMap<String, Bpmn>,
    starts: HashMap<String, String>,
    main_start: String,
    output_names: HashMap<String, HashMap<String, String>>,
    boundary_events: HashMap<String, HashMap<Symbol, String>>,
}

impl Process {
    /// Create new process and initialize it from the BPMN file path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        let (data, starts) = read_bpmn_file(path)?;
        let main_start = starts
            .values()
            .find(|start_id| start_id.to_lowercase().starts_with(STARTEVENT_PREFIX))
            .ok_or(Error::MissingProcessStart)?
            .clone();

        // Collect all referencing output names
        let mut output_names: HashMap<String, HashMap<String, String>> = HashMap::new();
        data.values().for_each(|bpmn| {
            if let Bpmn::SequenceFlow {
                id,
                name: Some(name),
                source_ref,
                target_ref: _,
            } = bpmn
            {
                let entry = output_names.entry(source_ref.to_string()).or_default();
                entry.insert(name.to_string(), id.to_string());
            }
        });

        // Collect all referencing boundarys
        let mut boundary_events: HashMap<String, HashMap<Symbol, String>> = HashMap::new();
        data.values().for_each(|bpmn| {
            if let Bpmn::Event {
                event: BpmnType::BoundaryEvent,
                symbol,
                id,
                name: _,
                attached_to_ref,
                cancel_activity: _,
                output: _,
            } = bpmn
            {
                if let Some(attached_to_ref) = attached_to_ref.as_ref() {
                    let entry = boundary_events
                        .entry(attached_to_ref.to_string())
                        .or_default();
                    // if no boundary symbol is set then use default.
                    entry.insert(symbol.clone().unwrap_or_default(), id.clone());
                }
            }
        });

        Ok(Self {
            data,
            starts,
            main_start,
            output_names,
            boundary_events,
        })
    }

    fn name_lookup(&self, gateway_id: &str, name: &str) -> Option<&String> {
        self.output_names
            .get(gateway_id)
            .and_then(|map| map.get(name))
    }

    fn boundary_lookup(&self, activity_id: &str, symbol: &Symbol) -> Option<&String> {
        self.boundary_events
            .get(activity_id)
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
                    let _ = handler.run_task(id, data.clone());
                }
                BpmnType::ExclusiveGateway
                | BpmnType::InclusiveGateway
                | BpmnType::ParallelGateway => {
                    let _ = handler.run_gateway(id, data.clone());
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
        self.execute(&self.main_start, handler, data.clone(), sender)?;

        // When sender die, the recv handle terminates.
        let traces = recv_handle.join().expect("oops! the child thread panicked");

        Ok(ProcessResult {
            result: Arc::try_unwrap(data).unwrap().into_inner().unwrap(),
            trace: traces,
        })
    }

    fn execute<'a, T>(
        &'a self,
        start_id: &'a String,
        handler: &Eventhandler<T>,
        data: Data<T>,
        sender: Sender<(BpmnType, String)>,
    ) -> ExecuteResult<'a>
    where
        T: Send + Sync,
    {
        let mut next_id = start_id;
        loop {
            let bpmn = self
                .data
                .get(next_id)
                .ok_or(Error::MissingId(next_id.to_string()))?;

            // We trace by name if exist. Id otherwise.
            let _ = sender.send((
                BpmnType::from(bpmn),
                bpmn.name()
                    .or_else(|| bpmn.id())
                    .cloned()
                    .unwrap_or_default(),
            ));

            next_id = match bpmn {
                Bpmn::Event {
                    event,
                    symbol: _,
                    id,
                    name,
                    attached_to_ref: _,
                    cancel_activity: _,
                    output,
                } => {
                    info!("{}: {}", event, name.as_ref().unwrap_or(id));
                    match event {
                        BpmnType::StartEvent
                        | BpmnType::IntermediateCatchEvent
                        | BpmnType::IntermediateThrowEvent
                        | BpmnType::BoundaryEvent => output
                            .as_ref()
                            .ok_or(Error::MissingOutput(event.to_string()))?,
                        BpmnType::EndEvent => break,
                        _ => return Err(Error::BadEventType),
                    }
                }
                Bpmn::Activity {
                    aktivity,
                    id,
                    name,
                    output,
                } => {
                    info!("{}: {}", aktivity, name.as_ref().unwrap_or(id));
                    match aktivity {
                        BpmnType::Task => {
                            let response =
                                handler.run_task(name.as_ref().unwrap_or(id), data.clone());
                            if let Err(symbol) = response {
                                self.boundary_lookup(id, &symbol)
                                    .ok_or(Error::MissingBoundary(
                                        name.as_ref().unwrap_or(id).clone(),
                                    ))?
                            } else {
                                output
                                    .as_ref()
                                    .ok_or(Error::MissingOutput(aktivity.to_string()))?
                            }
                        }
                        BpmnType::SubProcess => {
                            let response_id = self.execute(
                                self.starts.get(next_id).ok_or(Error::MissingProcessStart)?,
                                handler,
                                data.clone(),
                                sender.clone(),
                            )?;

                            if let Some(Bpmn::Event {
                                event: BpmnType::EndEvent,
                                symbol: Some(symbol),
                                id: _,
                                name: _,
                                attached_to_ref: _,
                                cancel_activity: _,
                                output: _,
                            }) = self.data.get(response_id)
                            {
                                self.boundary_lookup(id, symbol)
                                    .ok_or(Error::MissingBoundary(
                                        name.as_ref().unwrap_or(id).clone(),
                                    ))?
                            } else {
                                output
                                    .as_ref()
                                    .ok_or(Error::MissingOutput(aktivity.to_string()))?
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
                    info!("{}: {}", gateway, name.as_ref().unwrap_or(id));
                    match gateway {
                        BpmnType::ExclusiveGateway => {
                            if outputs.len() > 1 {
                                let name_or_id = name.as_ref().unwrap_or(id);
                                // Default to first outgoing if function is not set.
                                let response = handler.run_gateway(name_or_id, data.clone());
                                let response = response
                                    .first()
                                    .copied()
                                    .or(default.as_deref())
                                    .or_else(|| outputs.first().map(|x| x.as_str()))
                                    .ok_or(Error::MissingId(id.to_owned()))?;

                                // look up name to id or just use answer
                                self.name_lookup(id, response)
                                    .or_else(|| {
                                        outputs.iter().find(|&outgoing| outgoing == response)
                                    })
                                    .ok_or(Error::MissingOutput(gateway.to_string()))?
                            } else {
                                outputs
                                    .first()
                                    .ok_or(Error::MissingOutput(gateway.to_string()))?
                            }
                        }
                        BpmnType::InclusiveGateway => {
                            // Converging gateway. Synchronize
                            if outputs.len() <= 1 {
                                return outputs
                                    .first()
                                    .ok_or(Error::MissingOutput(gateway.to_string()));
                            }

                            // Diverging gateway
                            let mut response =
                                handler.run_gateway(name.as_ref().unwrap_or(id), data.clone());
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
                                    self.execute(outgoing, handler, data.clone(), sender.clone())
                                })
                                .partition(Result::is_ok);

                            if !errors.is_empty() {
                                if let Some(result) = errors.pop() {
                                    return result;
                                }
                            }

                            oks.into_iter().filter_map(Result::ok).next().ok_or(
                                Error::BadGatewayOutput(format!(
                                    "No output with name: {}",
                                    response.join(", ")
                                )),
                            )?
                        }
                        BpmnType::ParallelGateway => {
                            // Converging gateway. Synchronize
                            if outputs.len() <= 1 {
                                return outputs
                                    .first()
                                    .ok_or(Error::MissingOutput(gateway.to_string()));
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
                                                handler,
                                                data.clone(),
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
                    source_ref: _,
                    target_ref,
                } => {
                    info!("SequenceFlow: {}", name.as_ref().unwrap_or(id));
                    target_ref
                }
                _ => return Err(Error::BadDiagramType),
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
