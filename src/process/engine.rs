mod execute_handler;

use crate::{
    Process, Symbol,
    error::{AT_LEAST_TWO_OUTGOING, Error},
    model::{ActivityType, Bpmn, Event, EventType, Gateway, GatewayType, Id, With},
};
use execute_handler::ExecuteHandler;
use log::{info, warn};
use std::{borrow::Cow, collections::HashSet, sync::Arc};

use super::{Run, handler::Data};

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
    pub(super) fn execute<'a>(&'a self, data: ExecuteData<'a, T>) -> ExecuteResult<'a>
    where
        T: Send,
    {
        let mut end_event = None;
        let mut handler = ExecuteHandler::new(Cow::from(&[0]));
        loop {
            let all_tokens = handler.take();
            if all_tokens.is_empty() {
                return end_event.ok_or(Error::MissingEndEvent);
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
                                .map(|token| self.flow(token, &data))
                                .collect()
                        })
                        .collect::<Vec<_>>();
                    results.into_iter()
                }
                #[cfg(not(feature = "parallel"))]
                all_tokens
                    .iter()
                    .map(|tokens| tokens.iter().map(|token| self.flow(token, &data)))
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
                                    symbol: Some(Symbol::Terminate),
                                    ..
                                } => return Ok(event),
                                _ => {
                                    end_event.replace(event);
                                }
                            }
                            handler.consume_token(None);
                        }
                        Ok(Return::Fork(item)) => handler.pending(item),
                        Err(value) => return Err(value),
                    }
                }

                // Once all inputs have been merged for a gateway, then proceed with its outputs.
                // The gateway vector contains all the gateways involved. Right now we are using balanced diagram
                // and do not need to investigate further.
                if let Some(mut gateways) = handler.tokens_consumed()
                    && let Some(
                        gateway @ Gateway {
                            gateway_type,
                            outputs,
                            ..
                        },
                    ) = gateways.pop()
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
                            handler.pending(self.handle_inclusive_gateway(&data, gateway)?);
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
                                (Some(name), Some(symbol @ Symbol::Link)) => self
                                    .diagram
                                    .find_catch_link(name, symbol, data.process_id)?,
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
                    id: id @ Id { bpmn_id, local_id },
                    func_idx,
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_ref().unwrap_or(bpmn_id);
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
                                .and_then(|index| self.handler.run_task(index, data.user_data()))
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        activity_type.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })? {
                                Some(boundary) => self
                                    .diagram
                                    .find_boundary(
                                        id,
                                        boundary.name(),
                                        boundary.symbol(),
                                        data.process_data,
                                    )
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            boundary.to_string(),
                                            name_or_id.into(),
                                        )
                                    })?,
                                None => maybe_fork!(self, outputs, data, activity_type, name_or_id),
                            }
                        }
                        ActivityType::SubProcess => {
                            let sp_data = self
                                .diagram
                                .get_process(*local_id)
                                .ok_or_else(|| Error::MissingProcessData(bpmn_id.into()))?;

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
                            } = self.execute(ExecuteData::new(sp_data, id, data.user_data()))?
                            {
                                self.diagram
                                    .find_boundary(id, name.as_deref(), symbol, data.process_data)
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
                        _ if outputs.len() == 1 && inputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive if outputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive => {
                            match func_idx
                                .and_then(|index| {
                                    self.handler.run_exclusive(index, data.user_data())
                                })
                                .ok_or_else(|| {
                                    Error::MissingImplementation(
                                        gateway_type.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })? {
                                Some(value) => outputs
                                    .find_by_name_or_id(value, data.process_data)
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
                        GatewayType::Parallel | GatewayType::Inclusive if inputs.len() > 1 => {
                            return Ok(Return::Join(gateway));
                        }
                        GatewayType::Parallel => {
                            return Ok(Return::Fork(Cow::Borrowed(outputs.ids())));
                        }
                        GatewayType::Inclusive => {
                            return Ok(Return::Fork(self.handle_inclusive_gateway(data, gateway)?));
                        }
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
                                        gateway_type.to_string(),
                                        name_or_id.to_string(),
                                    )
                                })?;

                            outputs
                                .find_by_intermediate_event(&value, data.process_data)
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
        data: &ExecuteData<'a, T>,
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
                .find_by_name_or_id(value, data.process_data)
                .ok_or_else(|| {
                    Error::MissingOutput(gateway_type.to_string(), name_or_id.to_string())
                })
        };

        let value = match func_idx
            .and_then(|index| self.handler.run_inclusive(index, data.user_data()))
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

    fn user_data(&self) -> Data<T> {
        Arc::clone(&self.user_data)
    }
}
