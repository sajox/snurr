use std::{
    collections::{HashMap, VecDeque},
    sync::{mpsc::Sender, Arc},
};

use log::info;

use crate::{
    error::Error,
    model::{ActivityType, Bpmn, EventType, GatewayType},
    process::{parallel::parallelize_helper, replay},
    Data, Eventhandler, Process, Symbol,
};

use super::ExecuteResult;

impl Process {
    pub(super) fn execute<'a, T>(
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
                                parallelize_helper!(outputs.into(), recursion)
                            }
                            EventType::IntermediateThrow => {
                                match (name.as_ref(), symbol.as_ref()) {
                                    (Some(name), Some(symbol @ Symbol::Link)) => {
                                        self.catch_event_lookup(name, symbol)?
                                    }
                                    // Follow outputs for other throw events
                                    (Some(_), _) => {
                                        parallelize_helper!(outputs.into(), recursion)
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
                                    Ok(_) => parallelize_helper!(outputs.into(), recursion),
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
                                    _ => parallelize_helper!(outputs.into(), recursion),
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

                                parallelize_helper!(responses, recursion)
                            }
                            GatewayType::Parallel => parallelize_helper!(outputs.into(), recursion),
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
}
