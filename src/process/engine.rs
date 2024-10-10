use std::{
    collections::VecDeque,
    sync::{mpsc::Sender, Arc},
};

use log::info;

use crate::{
    error::Error,
    model::{ActivityType, Bpmn, EventType, GatewayType},
    process::{parallel::parallelize_helper, replay},
    Data, Eventhandler, Process, Symbol,
};

impl Process {
    pub(super) fn execute<'a, T>(
        &'a self,
        start_ids: Vec<&usize>,
        data: &ExecuteData<'a, T>,
    ) -> ExecuteResult<'_>
    where
        T: Send,
    {
        let mut results: Vec<&usize> = vec![];
        let mut queue = VecDeque::from(start_ids);
        while let Some(current_id) = queue.pop_front() {
            let next_id = match data
                .process_data
                .get(*current_id)
                .ok_or_else(|| Error::MisssingBpmnData(current_id.to_string()))?
            {
                Bpmn::Event {
                    event,
                    symbol,
                    id: (bpmn_id, _),
                    name,
                    outputs,
                    ..
                } => {
                    info!("{}: {}", event, name.as_ref().unwrap_or(bpmn_id));
                    match event {
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            parallelize_helper!(self, outputs.ids(), data)
                        }
                        EventType::IntermediateThrow => {
                            match (name.as_ref(), symbol.as_ref()) {
                                (Some(name), Some(symbol @ Symbol::Link)) => {
                                    self.catch_event_lookup(name, symbol)?
                                }
                                // Follow outputs for other throw events
                                (Some(_), _) => {
                                    parallelize_helper!(self, outputs.ids(), data)
                                }
                                _ => Err(Error::MissingIntermediateThrowEventName(bpmn_id.into()))?,
                            }
                        }
                        EventType::End => {
                            if let Some(boundary_id) = symbol
                                .as_ref()
                                .and_then(|symbol| self.boundary_lookup(data.process_id, symbol))
                            {
                                results.push(boundary_id);
                            }
                            continue;
                        }
                    }
                }
                Bpmn::Activity {
                    activity,
                    id,
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_ref().unwrap_or(id);
                    info!("{}: {}", activity, name_or_id);
                    match activity {
                        ActivityType::Task => {
                            data.trace(replay::TASK, name_or_id)?;
                            match data.handler.run_task(name_or_id, data.user_data()) {
                                Ok(_) => {
                                    parallelize_helper!(self, outputs.ids(), data)
                                }
                                Err(symbol) => {
                                    self.boundary_lookup(id, &symbol).ok_or_else(|| {
                                        Error::MissingBoundary(
                                            symbol.to_string(),
                                            name_or_id.into(),
                                        )
                                    })?
                                }
                            }
                        }
                        ActivityType::SubProcess => {
                            let (process_id, process_data) = self
                                .data
                                .get_key_value(id)
                                .ok_or_else(|| Error::MissingProcessData(id.into()))?;

                            match self
                                .execute(vec![&0], &data.update(process_id, process_data))?
                                .as_slice()
                            {
                                // Boundary id returned
                                [id, ..] => id,
                                // Continue from subprocess
                                _ => {
                                    parallelize_helper!(self, outputs.ids(), data)
                                }
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
                    data.trace(replay::GATEWAY, name_or_id)?;

                    let first = outputs.first().ok_or_else(|| {
                        Error::MissingOutput(gateway.to_string(), name_or_id.to_string())
                    })?;
                    match gateway {
                        GatewayType::Exclusive => first,
                        GatewayType::Inclusive | GatewayType::Parallel => {
                            results.push(first);
                            continue;
                        }
                    }
                }
                // Fork
                Bpmn::Gateway {
                    gateway,
                    id,
                    name,
                    default: (_, default_lid),
                    outputs,
                    ..
                } if outputs.len() > 1 => {
                    let name_or_id = name.as_ref().unwrap_or(id);
                    info!("{}: {}", gateway, name_or_id);
                    data.trace(replay::GATEWAY, name_or_id)?;

                    match gateway {
                        GatewayType::Exclusive => {
                            // Default to first outgoing if function is not set.
                            let responses = data.handler.run_gateway(name_or_id, data.user_data());
                            responses
                                .first()
                                .and_then(|response| outputs.name_to_id(response))
                                .or(default_lid.as_ref())
                                .ok_or_else(|| {
                                    Error::MissingDefault(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?
                        }
                        GatewayType::Inclusive => {
                            let responses = data.handler.run_gateway(name_or_id, data.user_data());
                            if responses.is_empty() {
                                default_lid.as_ref().ok_or_else(|| {
                                    Error::MissingDefault(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?
                            } else {
                                // Run all chosen paths
                                let responses: Vec<_> = responses
                                    .iter()
                                    .filter_map(|response| outputs.name_to_id(response))
                                    .collect();
                                parallelize_helper!(self, responses, data)
                            }
                        }
                        GatewayType::Parallel => {
                            parallelize_helper!(self, outputs.ids(), data)
                        }
                    }
                }
                Bpmn::SequenceFlow {
                    id,
                    name,
                    target_ref: (_, lid),
                    ..
                } => {
                    info!("SequenceFlow: {}", name.as_ref().unwrap_or(id));
                    lid
                }
                bpmn => return Err(Error::TypeNotImplemented(format!("{bpmn:?}"))),
            };
            queue.push_back(next_id);
        }
        Ok(results)
    }

    fn boundary_lookup(&self, activity_id: &str, symbol: &Symbol) -> Option<&usize> {
        self.activity_ids
            .get(activity_id)
            .and_then(|map| map.get(symbol))
    }

    fn catch_event_lookup(&self, throw_event_name: &str, symbol: &Symbol) -> Result<&usize, Error> {
        self.catch_events_ids
            .get(throw_event_name)
            .and_then(|map| map.get(symbol))
            .ok_or_else(|| {
                Error::MissingIntermediateCatchEvent(symbol.to_string(), throw_event_name.into())
            })
    }
}

pub(super) type ExecuteResult<'a> = Result<Vec<&'a usize>, Error>;

// Data for the execution engine.
pub(super) struct ExecuteData<'a, T> {
    process_id: &'a str,
    process_data: &'a Vec<Bpmn>,
    handler: &'a Eventhandler<T>,
    user_data: Data<T>,
    trace: Sender<(&'static str, String)>,
}

impl<'a, T> ExecuteData<'a, T> {
    pub(super) fn new(
        process_id: &'a str,
        process_data: &'a Vec<Bpmn>,
        handler: &'a Eventhandler<T>,
        user_data: Data<T>,
        trace: Sender<(&'static str, String)>,
    ) -> Self {
        Self {
            process_id,
            process_data,
            handler,
            user_data,
            trace,
        }
    }

    // When we change to a sub process we must change process id and data.
    fn update(&self, process_id: &'a str, process_data: &'a Vec<Bpmn>) -> Self {
        Self {
            process_id,
            process_data,
            handler: self.handler,
            user_data: self.user_data(),
            trace: self.trace.clone(),
        }
    }

    fn trace(&self, bpmn_type: &'static str, value: impl Into<String>) -> Result<(), Error> {
        Ok(self.trace.send((bpmn_type, value.into()))?)
    }

    fn user_data(&self) -> Data<T> {
        Arc::clone(&self.user_data)
    }
}
