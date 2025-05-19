mod bpmn_queue;

use crate::{
    IntermediateEvent, Process, Symbol,
    error::{AT_LEAST_TWO_OUTGOING, Error, TOO_MANY_END_SYMBOLS},
    model::{ActivityType, Bpmn, EventType, Gateway, GatewayType, Id, With},
};
use bpmn_queue::BpmnQueue;
use log::info;
use std::{borrow::Cow, ops::ControlFlow, sync::Arc};

use super::{Run, handler::Data};

#[derive(Debug)]
enum Return<'a> {
    Fork(Cow<'a, [usize]>),
    Join(&'a Gateway),
    End(Option<&'a Bpmn>),
}

macro_rules! maybe_fork {
    ($self:expr, $outputs:expr, $data:expr, $ty:expr, $noi:expr) => {
        if $outputs.len() <= 1 {
            $outputs
                .first()
                .ok_or_else(|| Error::MissingOutput($ty.to_string(), $noi.to_string()))?
        } else {
            return Ok(Return::Fork(Cow::Borrowed($outputs.ids())));
        }
    };
}

impl<T> Process<Run, T> {
    pub(super) fn execute<'a>(&'a self, data: &ExecuteData<'a, T>) -> ExecuteResult<'a>
    where
        T: Send,
    {
        let mut bpmn_end = None;
        let mut queue = BpmnQueue::new(Cow::from(&[0]));
        loop {
            let all_tokens = queue.take();
            if all_tokens.is_empty() {
                return Ok(bpmn_end);
            }

            let result_iter = {
                #[cfg(feature = "parallel")]
                {
                    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
                    let results: Vec<Vec<_>> = all_tokens
                        .par_iter()
                        .map(|tokens| {
                            tokens
                                .par_iter()
                                .map(|token| self.flow(token, data))
                                .collect()
                        })
                        .collect::<Vec<_>>();
                    results.into_iter()
                }
                #[cfg(not(feature = "parallel"))]
                all_tokens
                    .iter()
                    .map(|tokens| tokens.iter().map(|token| self.flow(token, data)))
            };

            for inner_iter in result_iter.rev() {
                // Correlate tokens that have arrived
                for result in inner_iter {
                    match result {
                        Ok(Return::Join(gateway)) => queue.join_token(gateway),
                        Ok(Return::End(value)) => {
                            if value.is_some_and(|symbol| bpmn_end.replace(symbol).is_some()) {
                                return Err(Error::BpmnRequirement(TOO_MANY_END_SYMBOLS.into()));
                            }
                            queue.end_token();
                        }
                        Ok(Return::Fork(item)) => queue.add_pending(item),
                        Err(value) => return Err(value),
                    }
                }

                // Once all inputs have been merged for a gateway, then proceed with its outputs.
                // The gateway vector contains all the gateways involved. Right now we are using balanced diagram
                // and do not need to investigate further.
                if let Some(mut gateways) = queue.tokens_consumed() {
                    if let Some(
                        gw @ Gateway {
                            gateway, outputs, ..
                        },
                    ) = gateways.pop()
                    {
                        // We cannot add new tokens until we have correlated all processed flows.
                        match gateway {
                            GatewayType::Inclusive if outputs.len() == 1 => {
                                queue.push_output(Cow::Borrowed(outputs.ids()));
                            }
                            // Handle Fork, the user code determine next token(s) to run.
                            GatewayType::Inclusive => {
                                let value = match self.handle_inclusive_gateway(data, gw)? {
                                    ControlFlow::Continue(value) => Cow::Owned(vec![*value]),
                                    ControlFlow::Break(value) => value,
                                };
                                queue.add_pending(value);
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Commit pending forks
            queue.commit();
        }
    }

    // Each flow process one "token" and returns on a Fork, Join or End.
    fn flow<'a: 'b, 'b>(
        &'a self,
        mut current_id: &'b usize,
        data: &ExecuteData<'a, T>,
    ) -> Result<Return<'a>, Error>
    where
        T: Send,
    {
        loop {
            current_id = match data
                .process_data
                .get(*current_id)
                .ok_or_else(|| Error::MisssingBpmnData(current_id.to_string()))?
            {
                bpmn @ Bpmn::Event {
                    event,
                    symbol,
                    id,
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    info!("{}: {}", event, name_or_id);
                    match event {
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            maybe_fork!(self, outputs, data, event, name_or_id)
                        }
                        EventType::IntermediateThrow => {
                            match (name.as_ref(), symbol.as_ref()) {
                                (Some(name), Some(symbol @ Symbol::Link)) => {
                                    self.catch_link_lookup(name, symbol, data.process_id)?
                                }
                                // Follow outputs for other throw events
                                (Some(_), _) => {
                                    maybe_fork!(self, outputs, data, event, name_or_id)
                                }
                                _ => {
                                    Err(Error::MissingIntermediateThrowEventName(id.bpmn().into()))?
                                }
                            }
                        }
                        EventType::End => {
                            if symbol.is_some() {
                                return Ok(Return::End(Some(bpmn)));
                            }
                            break;
                        }
                    }
                }
                Bpmn::Activity {
                    activity,
                    id: id @ Id { bpmn_id, local_id },
                    func_idx,
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_ref().unwrap_or(bpmn_id);
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
                            match func_idx
                                .and_then(|index| self.handler.run_task(index, data.user_data()))
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        activity.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })? {
                                Some(boundary) => self
                                    .boundary_lookup(
                                        bpmn_id,
                                        boundary.name(),
                                        boundary.symbol(),
                                        data.process_data,
                                    )
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            format!("{}", boundary),
                                            name_or_id.into(),
                                        )
                                    })?,
                                None => maybe_fork!(self, outputs, data, activity, name_or_id),
                            }
                        }
                        ActivityType::SubProcess => {
                            let sp_data = self
                                .diagram
                                .get_process(*local_id)
                                .ok_or_else(|| Error::MissingProcessData(bpmn_id.into()))?;

                            if let Some(Bpmn::Event {
                                event: EventType::End,
                                symbol: Some(symbol),
                                name,
                                ..
                            }) = self.execute(&data.update(id, sp_data))?
                            {
                                self.boundary_lookup(
                                    bpmn_id,
                                    name.as_deref(),
                                    symbol,
                                    data.process_data,
                                )
                                .ok_or_else(|| {
                                    Error::MissingBoundary(symbol.to_string(), name_or_id.into())
                                })?
                            } else {
                                // Continue from subprocess
                                maybe_fork!(self, outputs, data, activity, name_or_id)
                            }
                        }
                    }
                }

                Bpmn::Gateway(
                    gw @ Gateway {
                        gateway,
                        id,
                        func_idx,
                        name,
                        default,
                        outputs,
                        inputs,
                    },
                ) => {
                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    info!("{}: {}", gateway, name_or_id);
                    match gateway {
                        _ if outputs.len() == 0 => {
                            return Err(Error::MissingOutput(
                                gateway.to_string(),
                                name_or_id.to_string(),
                            ));
                        }
                        // Handle 1 to 1, probably a temporary design or mistake
                        _ if outputs.len() == 1 && inputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive if outputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive => {
                            match func_idx
                                .and_then(|index| {
                                    self.handler.run_exclusive(index, data.user_data())
                                })
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })? {
                                Some(value) => {
                                    output_by_name_or_id(value, outputs.ids(), data.process_data)
                                        .ok_or_else(|| {
                                            Error::MissingOutput(
                                                gateway.to_string(),
                                                name_or_id.to_string(),
                                            )
                                        })?
                                }
                                None => default_path(default, gateway, name_or_id)?,
                            }
                        }
                        // Handle a regular Join or a JoinFork. In both cases, we need to wait for all tokens.
                        GatewayType::Parallel | GatewayType::Inclusive if inputs.len() > 1 => {
                            return Ok(Return::Join(gw));
                        }
                        GatewayType::Parallel => {
                            return Ok(Return::Fork(Cow::Borrowed(outputs.ids())));
                        }
                        GatewayType::Inclusive => match self.handle_inclusive_gateway(data, gw)? {
                            ControlFlow::Continue(value) => value,
                            ControlFlow::Break(value) => return Ok(Return::Fork(value)),
                        },
                        GatewayType::EventBased if outputs.len() == 1 => {
                            return Err(Error::BpmnRequirement(AT_LEAST_TWO_OUTGOING.into()));
                        }
                        GatewayType::EventBased => {
                            let value = func_idx
                                .and_then(|index| {
                                    self.handler.run_event_based(index, data.user_data())
                                })
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?;

                            output_by_symbol(&value, outputs.ids(), data.process_data).ok_or_else(
                                || {
                                    Error::MissingIntermediateEvent(
                                        gateway.to_string(),
                                        name_or_id.to_string(),
                                        value.to_string(),
                                    )
                                },
                            )?
                        }
                    }
                }
                Bpmn::SequenceFlow {
                    id,
                    name,
                    target_ref,
                    ..
                } => {
                    info!("SequenceFlow: {}", name.as_deref().unwrap_or(id.bpmn()));
                    target_ref.local()
                }
                bpmn => return Err(Error::TypeNotImplemented(format!("{bpmn:?}"))),
            };
        }
        Ok(Return::End(None))
    }

    fn handle_inclusive_gateway<'a>(
        &'a self,
        data: &ExecuteData<'a, T>,
        Gateway {
            gateway,
            id,
            func_idx,
            name,
            default,
            outputs,
            ..
        }: &'a Gateway,
    ) -> Result<ControlFlow<Cow<'a, [usize]>, &'a usize>, Error> {
        let name_or_id = name.as_deref().unwrap_or(id.bpmn());
        let value = match func_idx
            .and_then(|index| self.handler.run_inclusive(index, data.user_data()))
            .ok_or_else(|| {
                Error::MissingImplementation(gateway.to_string(), name_or_id.to_string())
            })? {
            With::Flow(value) => output_by_name_or_id(value, outputs.ids(), data.process_data)
                .ok_or_else(|| Error::MissingOutput(gateway.to_string(), name_or_id.to_string()))?,

            With::Fork(value) => match value.as_slice() {
                [] => default_path(default, gateway, name_or_id)?,
                [one] => output_by_name_or_id(one, outputs.ids(), data.process_data).ok_or_else(
                    || Error::MissingOutput(gateway.to_string(), name_or_id.to_string()),
                )?,
                [..] => {
                    let outputs = outputs.ids();
                    return Ok(ControlFlow::Break(Cow::Owned(
                        value
                            .iter()
                            .filter_map(|&response| {
                                output_by_name_or_id(response, outputs, data.process_data)
                            })
                            .cloned()
                            .collect(),
                    )));
                }
            },
            With::Default => default_path(default, gateway, name_or_id)?,
        };

        Ok(ControlFlow::Continue(value))
    }

    fn boundary_lookup<'a>(
        &'a self,
        activity_id: &str,
        search_name: Option<&str>,
        search_symbol: &Symbol,
        process_data: &'a [Bpmn],
    ) -> Option<&'a usize> {
        self.diagram
            .boundaries
            .get(activity_id)?
            .iter()
            .filter_map(|index| process_data.get(*index))
            .filter_map(|bpmn| {
                // TODO rewrite with let chains when stable
                if let Bpmn::Event {
                    symbol, id, name, ..
                } = bpmn
                {
                    if symbol.as_ref() == Some(search_symbol) && search_name == name.as_deref() {
                        return Some(id.local());
                    }
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
        process_id: &Id,
    ) -> Result<&usize, Error> {
        self.diagram
            .catch_event_links
            .get(process_id.bpmn())
            .and_then(|links| links.get(throw_event_name))
            .ok_or_else(|| {
                Error::MissingIntermediateCatchEvent(symbol.to_string(), throw_event_name.into())
            })
    }
}

fn default_path<'a>(
    default: &'a Option<Id>,
    gateway: &GatewayType,
    name_or_id: &str,
) -> Result<&'a usize, Error> {
    default
        .as_ref()
        .map(Id::local)
        .ok_or_else(|| Error::MissingDefault(gateway.to_string(), name_or_id.to_string()))
}

fn output_by_symbol<'a>(
    search: &IntermediateEvent,
    outputs: &'a [usize],
    process_data: &'a [Bpmn],
) -> Option<&'a usize> {
    outputs.iter().find(|index| {
        process_data
            .get(**index)
            .and_then(|bpmn| {
                if let Bpmn::SequenceFlow { target_ref, .. } = bpmn {
                    return process_data.get(*target_ref.local());
                }
                None
            })
            .is_some_and(|bpmn| match bpmn {
                // We can target both ReceiveTask or Events.
                Bpmn::Activity {
                    activity,
                    name: Some(name),
                    ..
                } => {
                    *activity == ActivityType::ReceiveTask
                        && search.1 == Symbol::Message
                        && name.as_str() == search.0
                }
                Bpmn::Event {
                    symbol:
                        Some(
                            symbol @ (Symbol::Message
                            | Symbol::Signal
                            | Symbol::Timer
                            | Symbol::Conditional),
                        ),
                    name: Some(name),
                    ..
                } => *symbol == search.1 && name.as_str() == search.0,
                _ => false,
            })
    })
}

fn output_by_name_or_id<'a>(
    search: impl AsRef<str>,
    outputs: &'a [usize],
    process_data: &'a [Bpmn],
) -> Option<&'a usize> {
    outputs.iter().find(|index| {
        if let Some(Bpmn::SequenceFlow { id, name, .. }) = process_data.get(**index) {
            return name.as_deref().is_some_and(|name| name == search.as_ref())
                || id.bpmn() == search.as_ref();
        }
        false
    })
}

pub(super) type ExecuteResult<'a> = Result<Option<&'a Bpmn>, Error>;

// Data for the execution engine.
pub(super) struct ExecuteData<'a, T> {
    process_data: &'a Vec<Bpmn>,
    process_id: &'a Id,
    user_data: Data<T>,
}

impl<'a, T> ExecuteData<'a, T> {
    pub(super) fn new(process_data: &'a Vec<Bpmn>, process_id: &'a Id, user_data: Data<T>) -> Self {
        Self {
            process_data,
            process_id,
            user_data,
        }
    }

    // When we change to a sub process we must change process id and data.
    fn update(&self, process_id: &'a Id, process_data: &'a Vec<Bpmn>) -> Self {
        Self {
            process_data,
            process_id,
            user_data: self.user_data(),
        }
    }

    fn user_data(&self) -> Data<T> {
        Arc::clone(&self.user_data)
    }
}
