mod execute_handler;

use super::Run;
use crate::{
    IntermediateEvent, Process,
    api::{Exclusive, Inclusive, Task},
    bpmn::{Activity, ActivityType, Bpmn, Event, EventType, Gateway, GatewayType, Symbol},
    diagram::{Outputs, ProcessData},
    process::{DiagramErrorKind, RuntimeError, RuntimeErrorKind},
};
use execute_handler::ExecuteHandler;
use log::{debug, warn};
use std::{borrow::Cow, fmt::Display};

type Tokens<'a> = Cow<'a, [usize]>;

#[derive(Debug)]
enum Return<'a> {
    Fork(Tokens<'a>),
    Join(&'a Gateway),
    End(&'a Event),
}

macro_rules! maybe_fork {
    ($outputs:expr, $ty:expr) => {
        if $outputs.len() <= 1 {
            $outputs
                .first()
                .ok_or_else(|| DiagramErrorKind::MissingOutput($ty.to_string()))?
        } else {
            return Ok(Return::Fork(Cow::Borrowed($outputs.ids())));
        }
    };
}

impl<T> Process<T, Run> {
    pub(super) fn execute<'a>(
        &'a self,
        input: ExecuteInput<'a, T>,
    ) -> Result<&'a Event, RuntimeError>
    where
        T: Send + Sync,
    {
        let mut last_visited_end = None;
        let mut handler = ExecuteHandler::new(input.process.start());
        loop {
            let active_tokens = handler.active_tokens();
            if active_tokens.is_empty() {
                return last_visited_end.ok_or(DiagramErrorKind::MissingEndEvent.into());
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
                            match event {
                                // A subprocess end event, terminate early. The result is used by the subprocess to choose boundary (interrupting).
                                Event {
                                    event_type: EventType::End,
                                    symbol: Some(_),
                                    ..
                                } if input.is_subprocess => return Ok(event),

                                // Regular process
                                Event {
                                    event_type: EventType::End,
                                    symbol: Some(Symbol::Terminate | Symbol::Error),
                                    ..
                                } => return Ok(event),
                                _ => {
                                    last_visited_end.replace(event);
                                    handler.consume_token(None);
                                }
                            }
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
    ) -> Result<Return<'a>, RuntimeError>
    where
        T: Send + Sync,
    {
        loop {
            current_id = match input.process.get(*current_id).ok_or_else(|| {
                RuntimeErrorKind::Engine(format!(
                    "could not fetch bpmn data with index {}",
                    current_id
                ))
            })? {
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
                    debug!("{event}");
                    match event_type {
                        EventType::Start | EventType::IntermediateCatch | EventType::Boundary => {
                            maybe_fork!(outputs, event)
                        }
                        EventType::IntermediateThrow => {
                            match (name.as_ref(), symbol.as_ref()) {
                                (Some(name), Some(Symbol::Link)) => {
                                    input.process.events.catch_event_link(name)?
                                }
                                // Follow outputs for other throw events
                                (Some(_), _) => {
                                    maybe_fork!(outputs, event)
                                }
                                _ => Err(DiagramErrorKind::MissingIntermediateThrowEventName(
                                    id.bpmn().into(),
                                ))?,
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
                    debug!("{activity}");
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
                                .map(|index| self.handler.run_task(index, input.data))
                                .ok_or_else(|| {
                                    RuntimeErrorKind::Engine(format!(
                                        "missing function {:?}",
                                        activity
                                    ))
                                })?? {
                                Task::Boundary(name, symbol) => input
                                    .process
                                    .events
                                    .boundary(id, symbol, name.as_deref())
                                    .ok_or_else(|| {
                                        DiagramErrorKind::MissingBoundary(
                                            format!("({name:?},{symbol})"),
                                            activity.to_string(),
                                        )
                                    })?,
                                Task::Default => maybe_fork!(outputs, activity),
                                Task::Panic(e) => Err(RuntimeErrorKind::Panic(e))?,
                            }
                        }
                        ActivityType::SubProcess { data_index } => {
                            let subprocess = if let Some(index) = data_index
                                && let Some(process_data) = self.diagram.get_process(*index)
                            {
                                process_data
                            } else {
                                Err(RuntimeErrorKind::Engine(format!(
                                    "missing subprocess data with bpmn id {:?}",
                                    activity
                                )))?
                            };

                            if let Event {
                                event_type: EventType::End,
                                symbol:
                                    Some(
                                        symbol @ (Symbol::Cancel
                                        | Symbol::Compensation
                                        | Symbol::Error
                                        | Symbol::Escalation
                                        | Symbol::Message
                                        | Symbol::Signal),
                                    ),
                                name,
                                ..
                            } = self.execute(ExecuteInput::new(subprocess, true, input.data))?
                            {
                                // Jump to boundary
                                input
                                    .process
                                    .events
                                    .boundary(id, *symbol, name.as_deref())
                                    .ok_or_else(|| {
                                        DiagramErrorKind::MissingBoundary(
                                            symbol.to_string(),
                                            activity.to_string(),
                                        )
                                    })?
                            } else {
                                // Continue from subprocess
                                maybe_fork!(outputs, activity)
                            }
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
                    debug!("{gateway}");
                    match gateway_type {
                        _ if outputs.len() == 0 => {
                            Err(DiagramErrorKind::MissingOutput(gateway.to_string()))?
                        }
                        // Handle 1 to 1, probably a temporary design or mistake
                        _ if outputs.len() == 1 && *inputs == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive if outputs.len() == 1 => outputs.first().unwrap(),
                        GatewayType::Exclusive => {
                            match func_idx
                                .map(|index| self.handler.run_exclusive(index, input.data))
                                .ok_or_else(|| {
                                    RuntimeErrorKind::Engine(format!(
                                        "missing function {:?}",
                                        gateway
                                    ))
                                })?? {
                                Exclusive::Flow(value) => {
                                    input.find_flow(&value, outputs, gateway)?
                                }
                                Exclusive::Default => gateway.default_path()?,
                                Exclusive::Panic(e) => Err(RuntimeErrorKind::Panic(e))?,
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
                            Err(DiagramErrorKind::BpmnRequirement(
                                "Event gateway must have at least two outgoing sequence flows"
                                    .into(),
                            ))?
                        }
                        GatewayType::EventBased => {
                            match func_idx
                                .map(|index| self.handler.run_eventbased(index, input.data))
                                .ok_or_else(|| {
                                    RuntimeErrorKind::Engine(format!(
                                        "missing function {:?}",
                                        gateway
                                    ))
                                })?? {
                                IntermediateEvent::Throw(name, symbol) => input
                                    .process
                                    .find_by_intermediate_event(&name, symbol, outputs)
                                    .ok_or_else(|| {
                                        DiagramErrorKind::MissingIntermediateEvent(
                                            gateway.to_string(),
                                            format!("({name},{symbol})"),
                                        )
                                    })?,
                                IntermediateEvent::Panic(e) => Err(RuntimeErrorKind::Panic(e))?,
                            }
                        }
                    }
                }
                Bpmn::SequenceFlow {
                    id,
                    name,
                    target_ref,
                    ..
                } => {
                    debug!("SequenceFlow `{}`", name.as_deref().unwrap_or(id.bpmn()));
                    target_ref.local()
                }
                bpmn @ Bpmn::Process { .. } => Err(RuntimeErrorKind::Engine(format!(
                    "unexpected usage of {bpmn:?}"
                )))?,
            };
        }
    }

    fn handle_inclusive_gateway<'a>(
        &'a self,
        input: &ExecuteInput<'a, T>,
        gateway @ Gateway {
            func_idx, outputs, ..
        }: &'a Gateway,
    ) -> Result<Tokens<'a>, RuntimeError> {
        let value = match func_idx
            .map(|index| self.handler.run_inclusive(index, input.data))
            .ok_or_else(|| RuntimeErrorKind::Engine(format!("missing function {:?}", gateway)))??
        {
            Inclusive::Flow(value) => input.find_flow(&value, outputs, gateway)?,
            Inclusive::Fork(mut values) => match values.as_slice() {
                [] => gateway.default_path()?,
                [value] => input.find_flow(value, outputs, gateway)?,
                [..] => {
                    let len_before_dedup = values.len();
                    values.sort();
                    values.dedup();

                    if len_before_dedup != values.len() {
                        warn!("{gateway} used flow(s) multiple times. Discarded the duplicates.");
                    }

                    let result = values
                        .into_iter()
                        .map(|value| input.find_flow(&value, outputs, gateway).copied())
                        .collect::<Result<Vec<usize>, _>>();
                    return Ok(Cow::Owned(result?));
                }
            },
            Inclusive::Default => gateway.default_path()?,
            Inclusive::Panic(e) => Err(RuntimeErrorKind::Panic(e))?,
        };
        Ok(Cow::Owned(vec![*value]))
    }
}

// Data for the execution engine.
pub(super) struct ExecuteInput<'a, T> {
    process: &'a ProcessData,
    is_subprocess: bool,
    data: &'a T,
}

impl<'a, T> ExecuteInput<'a, T> {
    pub(super) fn new(process: &'a ProcessData, is_subprocess: bool, data: &'a T) -> Self {
        Self {
            process,
            is_subprocess,
            data,
        }
    }

    fn find_flow(
        &self,
        search: &str,
        outputs: &'a Outputs,
        message: impl Display,
    ) -> Result<&'a usize, RuntimeError> {
        Ok(self
            .process
            .find_by_name_or_id(search, outputs)
            .ok_or_else(|| DiagramErrorKind::MissingOutput(message.to_string()))?)
    }
}
