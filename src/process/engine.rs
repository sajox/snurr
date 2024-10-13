use std::{collections::VecDeque, sync::Arc};

#[cfg(feature = "trace")]
use crate::process::replay;
#[cfg(feature = "trace")]
use std::sync::mpsc::Sender;

use log::info;

use crate::{
    error::Error,
    model::{ActivityType, Bpmn, BpmnLocal, EventType, GatewayType, With},
    process::parallel::parallelize_helper,
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
                    id: BpmnLocal(bid, _),
                    name,
                    outputs,
                    ..
                } => {
                    info!("{}: {}", event, name.as_ref().unwrap_or(bid));
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
                                _ => Err(Error::MissingIntermediateThrowEventName(bid.into()))?,
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
                        ActivityType::Task
                        | ActivityType::ScriptTask
                        | ActivityType::UserTask
                        | ActivityType::ServiceTask
                        | ActivityType::CallActivity
                        | ActivityType::ReceiveTask
                        | ActivityType::SendTask
                        | ActivityType::ManualTask
                        | ActivityType::BusinessRuleTask => {
                            #[cfg(feature = "trace")]
                            data.trace(replay::TASK, name_or_id)?;
                            if let Err(symbol) = data.handler.run_task(name_or_id, data.user_data())
                            {
                                self.boundary_lookup(id, &symbol).ok_or_else(|| {
                                    Error::MissingBoundary(symbol.to_string(), name_or_id.into())
                                })?
                            } else {
                                parallelize_helper!(self, outputs.ids(), data)
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
                    #[cfg(feature = "trace")]
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
                        GatewayType::EventBased => {
                            return Err(Error::BpmnRequirement(
                                "Event gateway must have at least two outgoing sequence flows"
                                    .into(),
                            ))
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
                    #[cfg(feature = "trace")]
                    data.trace(replay::GATEWAY, name_or_id)?;

                    match gateway {
                        GatewayType::Exclusive | GatewayType::EventBased => {
                            let responses = data.handler.run_gateway(name_or_id, data.user_data());
                            if responses.is_empty() {
                                default.as_ref().map(BpmnLocal::local).ok_or_else(|| {
                                    Error::MissingDefault(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?
                            } else {
                                responses
                                    .first()
                                    .and_then(|response| {
                                        output_by(response, &outputs.ids(), data.process_data)
                                    })
                                    .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?
                            }
                        }
                        GatewayType::Inclusive => {
                            let responses = data.handler.run_gateway(name_or_id, data.user_data());
                            if responses.is_empty() {
                                default.as_ref().map(BpmnLocal::local).ok_or_else(|| {
                                    Error::MissingDefault(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?
                            } else {
                                let outputs = outputs.ids();
                                // Run all chosen paths
                                let responses: Vec<_> = responses
                                    .iter()
                                    .filter_map(|response| {
                                        output_by(response, &outputs, data.process_data)
                                    })
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
                    target_ref,
                    ..
                } => {
                    info!("SequenceFlow: {}", name.as_ref().unwrap_or(id));
                    target_ref.local()
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

fn output_by<'a>(
    search: &With,
    outputs: &[&'a usize],
    process_data: &'a [Bpmn],
) -> Option<&'a usize> {
    outputs
        .iter()
        .filter(|index| match search {
            With::Name(resp) => {
                if let Some(Bpmn::SequenceFlow { name, .. }) = process_data.get(***index) {
                    return name.as_deref() == Some(resp);
                }
                false
            }
            With::Id(resp) => {
                if let Some(Bpmn::SequenceFlow { id, .. }) = process_data.get(***index) {
                    return id == resp;
                }
                false
            }
            With::Symbol(search, search_symbol) => process_data
                .get(***index)
                .and_then(|bpmn| {
                    if let Bpmn::SequenceFlow { target_ref, .. } = bpmn {
                        return process_data.get(*target_ref.local());
                    }
                    None
                })
                .filter(|bpmn| match bpmn {
                    Bpmn::Activity {
                        id, activity, name, ..
                    } => {
                        activity == &ActivityType::ReceiveTask
                            && search_symbol == &Symbol::Message
                            && (search.filter(|&sn| sn == id).is_some()
                                || *search == name.as_deref())
                    }
                    Bpmn::Event {
                        id, symbol, name, ..
                    } => {
                        symbol.as_ref() == Some(search_symbol)
                            && (search.filter(|&sn| sn == id.bpmn()).is_some()
                                || *search == name.as_deref())
                    }
                    _ => false,
                })
                .is_some(),
        })
        .copied()
        .next()
}

pub(super) type ExecuteResult<'a> = Result<Vec<&'a usize>, Error>;

// Data for the execution engine.
pub(super) struct ExecuteData<'a, T> {
    process_id: &'a str,
    process_data: &'a Vec<Bpmn>,
    handler: &'a Eventhandler<T>,
    user_data: Data<T>,
    #[cfg(feature = "trace")]
    trace: Sender<(&'static str, String)>,
}

impl<'a, T> ExecuteData<'a, T> {
    pub(super) fn new(
        process_id: &'a str,
        process_data: &'a Vec<Bpmn>,
        handler: &'a Eventhandler<T>,
        user_data: Data<T>,
        #[cfg(feature = "trace")] trace: Sender<(&'static str, String)>,
    ) -> Self {
        Self {
            process_id,
            process_data,
            handler,
            user_data,
            #[cfg(feature = "trace")]
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
            #[cfg(feature = "trace")]
            trace: self.trace.clone(),
        }
    }

    #[cfg(feature = "trace")]
    fn trace(&self, bpmn_type: &'static str, value: impl Into<String>) -> Result<(), Error> {
        Ok(self.trace.send((bpmn_type, value.into()))?)
    }

    fn user_data(&self) -> Data<T> {
        Arc::clone(&self.user_data)
    }
}
