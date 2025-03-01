use std::{collections::VecDeque, sync::Arc};

#[cfg(feature = "trace")]
use crate::process::replay;
#[cfg(feature = "trace")]
use std::sync::mpsc::Sender;

use log::info;

use crate::{
    Data, Eventhandler, Process, Symbol,
    error::Error,
    model::{ActivityType, Bpmn, BpmnLocal, EventType, GatewayType, With},
    process::parallel::parallelize_helper,
};

impl Process {
    pub(super) fn execute<'a, T>(
        &'a self,
        start_ids: Vec<&usize>,
        data: &ExecuteData<'a, T>,
    ) -> ExecuteResult<'a>
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
                    id: BpmnLocal(bid, lid),
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_ref().unwrap_or(bid);
                    info!("{}: {}", event, name_or_id);
                    match event {
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            parallelize_helper!(self, outputs, data, event, name_or_id)
                        }
                        EventType::IntermediateThrow => {
                            match (name.as_ref(), symbol.as_ref()) {
                                (Some(name), Some(symbol @ Symbol::Link)) => {
                                    self.catch_link_lookup(name, symbol, data.process_id)?
                                }
                                // Follow outputs for other throw events
                                (Some(_), _) => {
                                    parallelize_helper!(self, outputs, data, event, name_or_id)
                                }
                                _ => Err(Error::MissingIntermediateThrowEventName(bid.into()))?,
                            }
                        }
                        EventType::End => {
                            if symbol.is_some() {
                                // return the End Id so parent can check info.
                                results.push(lid);
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
                            if let Some(boundary) =
                                data.handler.run_task(name_or_id, data.user_data())
                            {
                                self.boundary_lookup(id, boundary.0, &boundary.1, data.process_data)
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            format!("{}", boundary),
                                            name_or_id.into(),
                                        )
                                    })?
                            } else {
                                parallelize_helper!(self, outputs, data, activity, name_or_id)
                            }
                        }
                        ActivityType::SubProcess => {
                            let sp_data = self
                                .data
                                .get(id)
                                .ok_or_else(|| Error::MissingProcessData(id.into()))?;

                            match self
                                .execute(vec![&0], &data.update(id, sp_data))?
                                .as_slice()
                            {
                                // Boundary id returned. Fetch info from subprocess data as it don't exist in data.process_data
                                [end_id, ..] => {
                                    let Some(Bpmn::Event {
                                        symbol: Some(symbol),
                                        name,
                                        ..
                                    }) = sp_data.get(**end_id)
                                    else {
                                        return Err(Error::MissingId(end_id.to_string()));
                                    };

                                    self.boundary_lookup(
                                        id,
                                        name.as_deref(),
                                        symbol,
                                        data.process_data,
                                    )
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            symbol.to_string(),
                                            name_or_id.into(),
                                        )
                                    })?
                                }
                                // Continue from subprocess
                                _ => parallelize_helper!(self, outputs, data, activity, name_or_id),
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
                            ));
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
                        GatewayType::Exclusive
                        | GatewayType::EventBased
                        | GatewayType::Inclusive => {
                            let response = data.handler.run_gateway(name_or_id, data.user_data());
                            match response {
                                With::Flow(value)
                                    if matches!(
                                        gateway,
                                        GatewayType::Exclusive | GatewayType::Inclusive
                                    ) =>
                                {
                                    output_by_name_or_id(value, &outputs.ids(), data.process_data)
                                        .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?
                                }
                                With::Fork(value) if matches!(gateway, GatewayType::Inclusive) => {
                                    if value.is_empty() {
                                        default_path(default, gateway, name_or_id)?
                                    } else {
                                        let outputs = outputs.ids();
                                        let responses: Vec<_> = value
                                            .iter()
                                            .filter_map(|&response| {
                                                output_by_name_or_id(
                                                    response,
                                                    &outputs,
                                                    data.process_data,
                                                )
                                            })
                                            .collect();

                                        match responses.as_slice() {
                                            [] => {
                                                return Err(Error::MissingOutput(
                                                    gateway.to_string(),
                                                    name_or_id.to_string(),
                                                ));
                                            }
                                            [first] => first,
                                            [..] => {
                                                match self.maybe_parallelize(responses, data)? {
                                                    Some(val) => val,
                                                    None => continue,
                                                }
                                            }
                                        }
                                    }
                                }
                                With::Default => default_path(default, gateway, name_or_id)?,
                                With::Symbol(_, _)
                                    if matches!(gateway, GatewayType::EventBased) =>
                                {
                                    output_by_symbol(&response, &outputs.ids(), data.process_data)
                                        .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?
                                }
                                With::Flow(_) => {
                                    return Err(Error::BpmnRequirement(format!(
                                        "{gateway} cannot use conditional expression"
                                    )));
                                }
                                With::Fork(_) => {
                                    return Err(Error::BpmnRequirement(format!(
                                        "{gateway} cannot fork"
                                    )));
                                }
                                With::Symbol(_, _) => {
                                    return Err(Error::BpmnRequirement(format!(
                                        "{gateway} cannot do decision based on events"
                                    )));
                                }
                            }
                        }
                        GatewayType::Parallel => {
                            parallelize_helper!(self, outputs, data, gateway, name_or_id)
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

    fn boundary_lookup<'a>(
        &'a self,
        activity_id: &str,
        search_name: Option<&str>,
        search_symbol: &Symbol,
        process_data: &'a [Bpmn],
    ) -> Option<&'a usize> {
        self.boundaries
            .get(activity_id)?
            .iter()
            .filter_map(|index| process_data.get(*index))
            .filter_map(|bpmn| {
                let Bpmn::Event {
                    symbol, id, name, ..
                } = bpmn
                else {
                    return None;
                };
                if symbol.as_ref() == Some(search_symbol) && search_name == name.as_deref() {
                    return Some(id.local());
                }
                None
            })
            .next()
    }

    // Links in specified process.
    fn catch_link_lookup(
        &self,
        throw_event_name: &str,
        symbol: &Symbol,
        process_id: &str,
    ) -> Result<&usize, Error> {
        self.catch_event_links
            .get(process_id)
            .and_then(|links| links.get(throw_event_name))
            .ok_or_else(|| {
                Error::MissingIntermediateCatchEvent(symbol.to_string(), throw_event_name.into())
            })
    }
}

fn default_path<'a>(
    default: &'a Option<BpmnLocal>,
    gateway: &GatewayType,
    name_or_id: &String,
) -> Result<&'a usize, Error> {
    default
        .as_ref()
        .map(BpmnLocal::local)
        .ok_or_else(|| Error::MissingDefault(gateway.to_string(), name_or_id.to_string()))
}

fn output_by_symbol<'a>(
    search: &With,
    outputs: &[&'a usize],
    process_data: &'a [Bpmn],
) -> Option<&'a usize> {
    outputs
        .iter()
        .filter(|index| match search {
            With::Symbol(search, search_symbol) => process_data
                .get(***index)
                .and_then(|bpmn| {
                    if let Bpmn::SequenceFlow { target_ref, .. } = bpmn {
                        return process_data.get(*target_ref.local());
                    }
                    None
                })
                .filter(|bpmn| match bpmn {
                    // We can target both ReceiveTask or Events.
                    Bpmn::Activity {
                        id, activity, name, ..
                    } => {
                        activity == &ActivityType::ReceiveTask
                            && search_symbol == &Symbol::Message
                            && (search.filter(|&sn| sn == id).is_some()
                                || *search == name.as_deref())
                    }
                    Bpmn::Event {
                        id,
                        symbol:
                            Some(
                                symbol @ (Symbol::Message
                                | Symbol::Signal
                                | Symbol::Timer
                                | Symbol::Conditional),
                            ),
                        name,
                        ..
                    } => {
                        symbol == search_symbol
                            && (search.filter(|&sn| sn == id.bpmn()).is_some()
                                || *search == name.as_deref())
                    }
                    _ => false,
                })
                .is_some(),
            _ => false,
        })
        .copied()
        .next()
}

fn output_by_name_or_id<'a>(
    search: impl AsRef<str>,
    outputs: &[&'a usize],
    process_data: &'a [Bpmn],
) -> Option<&'a usize> {
    outputs
        .iter()
        .filter(|index| {
            if let Some(Bpmn::SequenceFlow { id, name, .. }) = process_data.get(***index) {
                return name
                    .as_deref()
                    .filter(|&name| name == search.as_ref())
                    .is_some()
                    || id == search.as_ref();
            }
            false
        })
        .copied()
        .next()
}

pub(super) type ExecuteResult<'a> = Result<Vec<&'a usize>, Error>;

// Data for the execution engine.
pub(super) struct ExecuteData<'a, T> {
    process_data: &'a Vec<Bpmn>,
    process_id: &'a str,
    handler: &'a Eventhandler<T>,
    user_data: Data<T>,
    #[cfg(feature = "trace")]
    trace: Sender<(&'static str, String)>,
}

impl<'a, T> ExecuteData<'a, T> {
    pub(super) fn new(
        process_data: &'a Vec<Bpmn>,
        process_id: &'a str,
        handler: &'a Eventhandler<T>,
        user_data: Data<T>,
        #[cfg(feature = "trace")] trace: Sender<(&'static str, String)>,
    ) -> Self {
        Self {
            process_data,
            process_id,
            handler,
            user_data,
            #[cfg(feature = "trace")]
            trace,
        }
    }

    // When we change to a sub process we must change process id and data.
    fn update(&self, process_id: &'a str, process_data: &'a Vec<Bpmn>) -> Self {
        Self {
            process_data,
            process_id,
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
