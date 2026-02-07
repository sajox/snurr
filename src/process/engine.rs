mod execute_handler;

use super::Run;
use crate::{
    Process,
    api::{Data, With},
    bpmn::{Activity, ActivityType, Bpmn, Event, EventType, Gateway, GatewayType, Symbol},
    diagram::ProcessData,
    error::{AT_LEAST_TWO_OUTGOING, Error},
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
    ($outputs:expr, $ty:expr) => {
        if $outputs.len() <= 1 {
            $outputs
                .first()
                .ok_or_else(|| Error::MissingOutput($ty.to_string()))?
        } else {
            return Ok(Return::Fork(Cow::Borrowed($outputs.ids())));
        }
    };
}

macro_rules! find_flow {
    ($outputs:expr, $value:expr, $input:expr, $ty:expr) => {
        $input
            .process
            .find_by_name_or_id($value, $outputs)
            .ok_or_else(|| Error::MissingOutput($ty.to_string()))
    };
}

impl<T> Process<T, Run> {
    pub(super) fn execute<'a>(&'a self, input: ExecuteInput<'a, T>) -> Result<&'a Event, Error>
    where
        T: Send,
    {
        let mut last_visited_end = None;
        let start = [input.process.start().ok_or(Error::MissingStartEvent)?];
        let mut handler = ExecuteHandler::new(Cow::from(&start));
        loop {
            let active_tokens = handler.active_tokens();
            if active_tokens.is_empty() {
                return last_visited_end.ok_or(Error::MissingEndEvent);
            }

            let flows_iter = {
                #[cfg(feature = "parallel")]
                {
                    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
                    let results: Vec<Vec<_>> = active_tokens
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
                active_tokens
                    .iter()
                    .map(|tokens| tokens.iter().map(|token| self.flow(token, &input)))
            };

            for flows_result in flows_iter.rev() {
                for flow_result in flows_result {
                    match flow_result {
                        Ok(Return::Join(gateway)) => handler.consume_token(Some(gateway)),
                        Ok(Return::End(event)) => {
                            if let Event {
                                event_type: EventType::End,
                                symbol: Some(Symbol::Terminate | Symbol::Cancel),
                                ..
                            } = event
                            {
                                return Ok(event);
                            }
                            last_visited_end.replace(event);
                            handler.consume_token(None);
                        }
                        Ok(Return::Fork(item)) => handler.pending_fork(item),
                        Err(value) => return Err(value),
                    }
                }

                // Check if all inputs have been merged for a gateway, then proceed with its outputs.
                if let Some(
                    gateway @ Gateway {
                        gateway_type,
                        outputs,
                        ..
                    },
                ) = handler.tokens_consumed()?
                {
                    match gateway_type {
                        GatewayType::Parallel | GatewayType::Inclusive if outputs.len() == 1 => {
                            handler.immediate(Cow::Borrowed(outputs.ids()));
                        }
                        GatewayType::Parallel => {
                            handler.pending_fork(Cow::Borrowed(outputs.ids()));
                        }
                        GatewayType::Inclusive => {
                            handler.pending_fork(self.handle_inclusive_gateway(&input, gateway)?);
                        }
                        _ => {}
                    }
                }
            }
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
                    info!("{event}");
                    match event_type {
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            maybe_fork!(outputs, event)
                        }
                        EventType::IntermediateThrow => {
                            match (name.as_ref(), symbol.as_ref()) {
                                (Some(name), Some(Symbol::Link)) => {
                                    input.process.catch_event_link(name)?
                                }
                                // Follow outputs for other throw events
                                (Some(_), _) => {
                                    maybe_fork!(outputs, event)
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
                Bpmn::Activity(
                    activity @ Activity {
                        activity_type,
                        id,
                        func_idx,
                        outputs,
                        ..
                    },
                ) => {
                    info!("{activity}");
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
                                .map(|index| self.handler.run_task(index, input.user_data()))
                                .ok_or_else(|| {
                                    Error::MissingImplementation(activity.to_string())
                                })?? {
                                Some(boundary) => input
                                    .process
                                    .find_boundary(id, boundary.name(), boundary.symbol())
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            boundary.to_string(),
                                            activity.to_string(),
                                        )
                                    })?,
                                None => maybe_fork!(outputs, activity),
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
                                            activity.to_string(),
                                        )
                                    })?
                            } else {
                                // Continue from subprocess
                                maybe_fork!(outputs, activity)
                            }
                        }
                        ActivityType::SubProcess { .. } => {
                            return Err(Error::MissingProcessData(activity.to_string()));
                        }
                    }
                }

                Bpmn::Gateway(
                    gateway @ Gateway {
                        gateway_type,
                        func_idx,
                        outputs,
                        inputs,
                        ..
                    },
                ) => {
                    info!("{gateway}");
                    match gateway_type {
                        _ if outputs.len() == 0 => {
                            return Err(Error::MissingOutput(gateway.to_string()));
                        }
                        // Handle 1 to 1, probably a temporary design or mistake
                        _ if outputs.len() == 1 && *inputs == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive if outputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive => {
                            match func_idx
                                .map(|index| self.handler.run_exclusive(index, input.user_data()))
                                .ok_or_else(|| {
                                    Error::MissingImplementation(gateway.to_string())
                                })?? {
                                Some(value) => find_flow!(outputs, value, input, gateway)?,
                                None => gateway.default_path()?,
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
                                .map(|index| self.handler.run_eventbased(index, input.user_data()))
                                .ok_or_else(|| {
                                    Error::MissingImplementation(gateway.to_string())
                                })??;

                            input
                                .process
                                .find_by_intermediate_event(&value, outputs)
                                .ok_or_else(|| {
                                    Error::MissingIntermediateEvent(
                                        gateway.to_string(),
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
                    info!(r#"SequenceFlow "{}""#, name.as_deref().unwrap_or(id.bpmn()));
                    target_ref.local()
                }
                bpmn => return Err(Error::TypeNotImplemented(format!("{bpmn:?}"))),
            };
        }
    }

    fn handle_inclusive_gateway<'a>(
        &'a self,
        input: &ExecuteInput<'a, T>,
        gateway @ Gateway {
            func_idx, outputs, ..
        }: &'a Gateway,
    ) -> Result<Cow<'a, [usize]>, Error> {
        let value = match func_idx
            .map(|index| self.handler.run_inclusive(index, input.user_data()))
            .ok_or_else(|| Error::MissingImplementation(gateway.to_string()))??
        {
            With::Flow(value) => find_flow!(outputs, value, input, gateway)?,
            With::Fork(values) => match values.as_slice() {
                [] => gateway.default_path()?,
                [value] => find_flow!(outputs, value, input, gateway)?,
                [..] => {
                    let mut tokens = HashSet::with_capacity(values.len());
                    for &value in values.iter() {
                        // Breaks on first error
                        if !tokens.insert(*find_flow!(outputs, value, input, gateway)?) {
                            // The flow has already been used, we just log an warning and continue.
                            warn!(
                                "{gateway} used flow {value} multiple times. Discarded the duplicates."
                            );
                        }
                    }
                    return Ok(Cow::Owned(tokens.into_iter().collect()));
                }
            },
            With::Default => gateway.default_path()?,
        };
        Ok(Cow::Owned(vec![*value]))
    }
}

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
