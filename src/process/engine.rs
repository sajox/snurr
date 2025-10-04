mod execute_handler;

use super::{Run, handler::Data};
use crate::{
    Process, Symbol,
    error::{AT_LEAST_TWO_OUTGOING, Error},
    model::{ActivityType, Bpmn, Event, EventType, Gateway, GatewayType, Id, With},
    process::{handler::CallbackResult, reader::ProcessData},
};
use execute_handler::ExecuteHandler;
use log::{info, warn};
use std::{borrow::Cow, collections::HashSet, sync::Arc};

#[derive(Debug)]
enum Return<'a> {
    Fork(Cow<'a, [usize]>),
    Join(&'a Gateway),
    End(&'a Event),
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

impl<T> Process<T, Run> {
    pub(super) fn execute<'a>(&'a self, input: ExecuteInput<'a, T>) -> ExecuteResult<'a>
    where
        T: Send,
    {
        let mut last_visited_end = None;
        let start = [input.process.start().ok_or(Error::MissingStartEvent)?];
        let mut handler = ExecuteHandler::new(Cow::from(&start));
        loop {
            let all_tokens = handler.take();
            if all_tokens.is_empty() {
                return last_visited_end.ok_or(Error::MissingEndEvent);
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
                                .map(|token| self.flow(token, &input))
                                .collect()
                        })
                        .collect::<Vec<_>>();
                    results.into_iter()
                }
                #[cfg(not(feature = "parallel"))]
                all_tokens
                    .iter()
                    .map(|tokens| tokens.iter().map(|token| self.flow(token, &input)))
            };

            for inner_iter in result_iter.rev() {
                // Correlate tokens that have arrived
                for result in inner_iter {
                    match result {
                        Ok(Return::Join(gateway)) => handler.consume_token(Some(gateway)),
                        Ok(Return::End(event)) => {
                            match event {
                                Event {
                                    event_type: EventType::End,
                                    symbol: Some(Symbol::Terminate | Symbol::Cancel),
                                    ..
                                } => return Ok(event),
                                _ => {
                                    last_visited_end.replace(event);
                                }
                            }
                            handler.consume_token(None);
                        }
                        Ok(Return::Fork(item)) => handler.pending(item),
                        Err(value) => return Err(value),
                    }
                }

                // Once all inputs have been merged for a gateway, then proceed with its outputs.
                if let Some(
                    gateway @ Gateway {
                        gateway_type,
                        outputs,
                        ..
                    },
                ) = handler.tokens_consumed()?
                {
                    // We cannot add new tokens until we have correlated all processed flows.
                    match gateway_type {
                        GatewayType::Parallel | GatewayType::Inclusive if outputs.len() == 1 => {
                            handler.immediate(Cow::Borrowed(outputs.ids()));
                        }
                        GatewayType::Parallel => {
                            handler.pending(Cow::Borrowed(outputs.ids()));
                        }
                        // Handle Fork, the user code determine next token(s) to run.
                        GatewayType::Inclusive => {
                            handler.pending(self.handle_inclusive_gateway(&input, gateway)?);
                        }
                        _ => {}
                    }
                }
            }
            // Commit pending forks
            handler.commit();
        }
    }

    // Each flow process one "token" and returns on a Fork, Join or End.
    fn flow<'a: 'b, 'b>(
        &'a self,
        mut current_id: &'b usize,
        input: &ExecuteInput<'a, T>,
    ) -> Result<Return<'a>, Error>
    where
        T: Send,
    {
        loop {
            current_id = match input
                .process
                .get(*current_id)
                .ok_or_else(|| Error::MisssingBpmnData(current_id.to_string()))?
            {
                Bpmn::Event(
                    event @ Event {
                        event_type,
                        symbol,
                        id,
                        name,
                        outputs,
                        ..
                    },
                ) => {
                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    info!("{event_type}: {name_or_id}");
                    match event_type {
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            maybe_fork!(self, outputs, data, event_type, name_or_id)
                        }
                        EventType::IntermediateThrow => {
                            match (name.as_ref(), symbol.as_ref()) {
                                (Some(name), Some(Symbol::Link)) => {
                                    input.process.catch_event_link(name)?
                                }
                                // Follow outputs for other throw events
                                (Some(_), _) => {
                                    maybe_fork!(self, outputs, data, event_type, name_or_id)
                                }
                                _ => {
                                    Err(Error::MissingIntermediateThrowEventName(id.bpmn().into()))?
                                }
                            }
                        }
                        EventType::End => {
                            return Ok(Return::End(event));
                        }
                    }
                }
                Bpmn::Activity {
                    activity_type,
                    id,
                    func_idx,
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    info!("{activity_type}: {name_or_id}");
                    match activity_type {
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
                                .map(|index| match self.handler.run(index, input.user_data()) {
                                    Some(CallbackResult::Task(result)) => result,
                                    _ => None,
                                })
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        activity_type.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })? {
                                Some(boundary) => input
                                    .process
                                    .find_boundary(id, boundary.name(), boundary.symbol())
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            boundary.to_string(),
                                            name_or_id.into(),
                                        )
                                    })?,
                                None => maybe_fork!(self, outputs, data, activity_type, name_or_id),
                            }
                        }
                        ActivityType::SubProcess {
                            data_index: Some(index),
                        } => {
                            let sp_data = self
                                .diagram
                                .get_process(*index)
                                .ok_or_else(|| Error::MissingProcessData(id.bpmn().into()))?;

                            if let Event {
                                event_type: EventType::End,
                                symbol:
                                    Some(
                                        symbol @ (Symbol::Cancel
                                        | Symbol::Compensation
                                        | Symbol::Conditional
                                        | Symbol::Error
                                        | Symbol::Escalation
                                        | Symbol::Message
                                        | Symbol::Signal
                                        | Symbol::Timer),
                                    ),
                                name,
                                ..
                            } = self.execute(ExecuteInput::new(sp_data, input.user_data()))?
                            {
                                input
                                    .process
                                    .find_boundary(id, name.as_deref(), symbol)
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            symbol.to_string(),
                                            name_or_id.into(),
                                        )
                                    })?
                            } else {
                                // Continue from subprocess
                                maybe_fork!(self, outputs, data, activity_type, name_or_id)
                            }
                        }
                        ActivityType::SubProcess { .. } => {
                            return Err(Error::MissingProcessData(
                                name.as_deref().unwrap_or(id.bpmn()).into(),
                            ));
                        }
                    }
                }

                Bpmn::Gateway(
                    gateway @ Gateway {
                        gateway_type,
                        id,
                        func_idx,
                        name,
                        default,
                        outputs,
                        inputs,
                    },
                ) => {
                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    info!("{gateway_type}: {name_or_id}");
                    match gateway_type {
                        _ if outputs.len() == 0 => {
                            return Err(Error::MissingOutput(
                                gateway_type.to_string(),
                                name_or_id.to_string(),
                            ));
                        }
                        // Handle 1 to 1, probably a temporary design or mistake
                        _ if outputs.len() == 1 && *inputs == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive if outputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive => {
                            match func_idx
                                .map(|index| match self.handler.run(index, input.user_data()) {
                                    Some(CallbackResult::Exclusive(result)) => result,
                                    _ => None,
                                })
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        gateway_type.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })? {
                                Some(value) => outputs
                                    .find_by_name_or_id(value, input.process.data())
                                    .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway_type.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?,
                                None => default_path(default, gateway_type, name_or_id)?,
                            }
                        }
                        // Handle a regular Join or a JoinFork. In both cases, we need to wait for all tokens.
                        GatewayType::Parallel | GatewayType::Inclusive if *inputs > 1 => {
                            return Ok(Return::Join(gateway));
                        }
                        GatewayType::Parallel => {
                            return Ok(Return::Fork(Cow::Borrowed(outputs.ids())));
                        }
                        GatewayType::Inclusive => {
                            return Ok(Return::Fork(
                                self.handle_inclusive_gateway(input, gateway)?,
                            ));
                        }
                        GatewayType::EventBased if outputs.len() == 1 => {
                            return Err(Error::BpmnRequirement(AT_LEAST_TWO_OUTGOING.into()));
                        }
                        GatewayType::EventBased => {
                            let value = func_idx
                                .and_then(|index| {
                                    match self.handler.run(index, input.user_data()) {
                                        Some(CallbackResult::EventBased(result)) => Some(result),
                                        _ => None,
                                    }
                                })
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        gateway_type.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?;

                            outputs
                                .find_by_intermediate_event(&value, input.process.data())
                                .ok_or_else(|| {
                                    Error::MissingIntermediateEvent(
                                        gateway_type.to_string(),
                                        name_or_id.to_string(),
                                        value.to_string(),
                                    )
                                })?
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
    }

    fn handle_inclusive_gateway<'a>(
        &'a self,
        input: &ExecuteInput<'a, T>,
        Gateway {
            gateway_type,
            id,
            func_idx,
            name,
            default,
            outputs,
            ..
        }: &'a Gateway,
    ) -> Result<Cow<'a, [usize]>, Error> {
        let name_or_id = name.as_deref().unwrap_or(id.bpmn());
        let find_flow = |value| {
            outputs
                .find_by_name_or_id(value, input.process.data())
                .ok_or_else(|| {
                    Error::MissingOutput(gateway_type.to_string(), name_or_id.to_string())
                })
        };

        let value = match func_idx
            .and_then(|index| match self.handler.run(index, input.user_data()) {
                Some(CallbackResult::Inclusive(result)) => Some(result),
                _ => None,
            })
            .ok_or_else(|| {
                Error::MissingImplementation(gateway_type.to_string(), name_or_id.to_string())
            })? {
            With::Flow(value) => find_flow(value)?,
            With::Fork(values) => match values.as_slice() {
                [] => default_path(default, gateway_type, name_or_id)?,
                [value] => find_flow(value)?,
                [..] => {
                    let mut outputs = HashSet::with_capacity(values.len());
                    for &value in values.iter() {
                        // Breaks on first error
                        if !outputs.insert(*find_flow(value)?) {
                            // The flow has already been used, we just log an warning and continue.
                            warn!(
                                "Inclusive Gateway {name_or_id} used flow {value} multiple times. Discarded the duplicates."
                            );
                        }
                    }
                    return Ok(Cow::Owned(outputs.into_iter().collect()));
                }
            },
            With::Default => default_path(default, gateway_type, name_or_id)?,
        };
        Ok(Cow::Owned(vec![*value]))
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

pub(super) type ExecuteResult<'a> = Result<&'a Event, Error>;

// Data for the execution engine.
pub(super) struct ExecuteInput<'a, T> {
    process: &'a ProcessData,
    user_data: Data<T>,
}

impl<'a, T> ExecuteInput<'a, T> {
    pub(super) fn new(process: &'a ProcessData, user_data: Data<T>) -> Self {
        Self { process, user_data }
    }

    fn user_data(&self) -> Data<T> {
        Arc::clone(&self.user_data)
    }
}
