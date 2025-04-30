mod token_stack;

use std::sync::Arc;

#[cfg(feature = "trace")]
use crate::process::replay;
#[cfg(feature = "trace")]
use std::sync::mpsc::Sender;

use log::info;
use token_stack::TokenStack;

use crate::{
    Data, Eventhandler, Process, Symbol,
    error::{
        AT_LEAST_TWO_OUTGOING, Error, cannot_do_events, cannot_fork, cannot_use_cond_expr,
        cannot_use_default,
    },
    model::{ActivityType, Bpmn, BpmnLocal, EventType, GatewayType, With},
};

#[derive(Debug)]
enum Return<'a> {
    Fork(Vec<&'a usize>),
    Join(Vec<&'a usize>),
    JoinFork(Vec<&'a usize>),
    End(Option<&'a Bpmn>),
}

macro_rules! maybe_fork {
    ($self:expr, $outputs:expr, $data:expr, $ty:expr, $noi:expr) => {
        if $outputs.len() <= 1 {
            $outputs
                .first()
                .ok_or_else(|| Error::MissingOutput($ty.to_string(), $noi.to_string()))?
        } else {
            return Ok(Return::Fork($outputs.ids()));
        }
    };
}

impl Process {
    pub(super) fn execute<'a, T>(&'a self, data: &ExecuteData<'a, T>) -> ExecuteResult<'a>
    where
        T: Send,
    {
        let mut token_stack = TokenStack::default();
        // Start event always begin on zero in every (sub) process.
        let mut bpmn_queue = vec![vec![&0]];
        // Add one token as we just added zero
        token_stack.push(1);

        while let Some(start_ids) = bpmn_queue.pop() {
            let tokens = start_ids.len();

            // Run flow single or multi threaded
            let (join_and_ends, forks): (Vec<_>, Vec<_>) = {
                let (oks, mut errors): (Vec<_>, Vec<_>) = {
                    #[cfg(feature = "parallel")]
                    {
                        use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
                        start_ids
                            .par_iter()
                            .map(|output| self.flow(output, data))
                            .partition(Result::is_ok)
                    }

                    #[cfg(not(feature = "parallel"))]
                    start_ids
                        .iter()
                        .map(|output| self.flow(output, data))
                        .partition(Result::is_ok)
                };

                if let Some(result) = errors.pop() {
                    result?;
                }

                oks.into_iter().filter_map(Result::ok).partition(|a| {
                    matches!(a, Return::End(_) | Return::Join(_) | Return::JoinFork(_))
                })
            };

            let tokens_to_reduce = join_and_ends.len();
            let mut join_ids = None;
            let mut is_join_fork = false;
            for result in join_and_ends {
                match result {
                    Return::Join(output_id) => join_ids.get_or_insert(vec![]).push(output_id),
                    Return::JoinFork(output_id) => {
                        is_join_fork = true;
                        join_ids.get_or_insert(vec![]).push(output_id);
                    }
                    // Currently only a subprocess use the end symbol and should not end with multiple tokens.
                    Return::End(symbol @ Some(_)) if bpmn_queue.is_empty() && tokens == 1 => {
                        return Ok(symbol);
                    }
                    _ => {}
                }
            }

            if let Some(mut output_ids) = join_ids.take_if(|_| token_stack.remove(tokens_to_reduce))
            {
                let tokens = output_ids.pop().unwrap();
                // If it is a join fork, add tokens as we just consumed all.
                if is_join_fork {
                    token_stack.push(tokens.len());
                }
                bpmn_queue.push(tokens);
            } else {
                token_stack.remove(tokens_to_reduce);
            }

            // Add fork flows after every Join or End. Oterwise we might add a Fork, and a Join reduce wrong token in the queue.
            for fork in forks {
                if let Return::Fork(vec) = fork {
                    token_stack.push(vec.len());
                    bpmn_queue.push(vec);
                }
            }
        }
        Ok(None)
    }

    // Each flow process one "token" and returns on a Fork, Join or End.
    fn flow<'a, T>(
        &'a self,
        start_id: &usize,
        data: &ExecuteData<'a, T>,
    ) -> Result<Return<'a>, Error>
    where
        T: Send,
    {
        let mut current_id = start_id;
        loop {
            current_id = match data
                .process_data
                .get(*current_id)
                .ok_or_else(|| Error::MisssingBpmnData(current_id.to_string()))?
            {
                bpmn @ Bpmn::Event {
                    event,
                    symbol,
                    id: BpmnLocal(bid, _),
                    name,
                    outputs,
                    ..
                } => {
                    let name_or_id = name.as_ref().unwrap_or(bid);
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
                                _ => Err(Error::MissingIntermediateThrowEventName(bid.into()))?,
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
                                maybe_fork!(self, outputs, data, activity, name_or_id)
                            }
                        }
                        ActivityType::SubProcess => {
                            let sp_data = self
                                .data
                                .get(id)
                                .ok_or_else(|| Error::MissingProcessData(id.into()))?;

                            if let Some(Bpmn::Event {
                                event: EventType::End,
                                symbol: Some(symbol),
                                name,
                                ..
                            }) = self.execute(&data.update(id, sp_data))?
                            {
                                self.boundary_lookup(id, name.as_deref(), symbol, data.process_data)
                                    .ok_or_else(|| {
                                        Error::MissingBoundary(
                                            symbol.to_string(),
                                            name_or_id.into(),
                                        )
                                    })?
                            } else {
                                // Continue from subprocess
                                maybe_fork!(self, outputs, data, activity, name_or_id)
                            }
                        }
                    }
                }

                Bpmn::Gateway {
                    gateway,
                    id,
                    name,
                    default,
                    outputs,
                    inputs,
                } => {
                    let name_or_id = name.as_ref().unwrap_or(id);
                    info!("{}: {}", gateway, name_or_id);
                    #[cfg(feature = "trace")]
                    data.trace(replay::GATEWAY, name_or_id)?;

                    match (gateway, outputs.len()) {
                        (_, 0) => {
                            return Err(Error::MissingOutput(
                                gateway.to_string(),
                                name_or_id.to_string(),
                            ));
                        }
                        (GatewayType::Exclusive, 1) => outputs.first().unwrap(),
                        (GatewayType::Inclusive | GatewayType::Parallel, 1) => {
                            return Ok(Return::Join(outputs.ids()));
                        }
                        (GatewayType::EventBased, 1) => {
                            return Err(Error::BpmnRequirement(AT_LEAST_TWO_OUTGOING.into()));
                        }
                        (GatewayType::Exclusive, _) => {
                            let response = data.handler.run_gateway(name_or_id, data.user_data());
                            match response {
                                With::Flow(value) => {
                                    output_by_name_or_id(value, &outputs.ids(), data.process_data)
                                        .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?
                                }
                                With::Default => default_path(default, gateway, name_or_id)?,
                                With::Fork(_) => return Err(cannot_fork(gateway)),
                                With::Symbol(_, _) => return Err(cannot_do_events(gateway)),
                            }
                        }
                        (GatewayType::Inclusive, _) => {
                            let response = data.handler.run_gateway(name_or_id, data.user_data());
                            match response {
                                With::Flow(value) => {
                                    output_by_name_or_id(value, &outputs.ids(), data.process_data)
                                        .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?
                                }
                                With::Fork(value) => {
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

                                        if responses.len() <= 1 {
                                            responses.first().ok_or_else(|| {
                                                Error::MissingOutput(
                                                    gateway.to_string(),
                                                    name_or_id.to_string(),
                                                )
                                            })?
                                        } else {
                                            return match inputs.len() {
                                                1 => Ok(Return::Fork(responses)),
                                                _ => Ok(Return::JoinFork(responses)),
                                            };
                                        }
                                    }
                                }
                                With::Default => default_path(default, gateway, name_or_id)?,
                                With::Symbol(_, _) => return Err(cannot_do_events(gateway)),
                            }
                        }
                        (GatewayType::EventBased, _) => {
                            let response = data.handler.run_gateway(name_or_id, data.user_data());
                            match response {
                                With::Symbol(_, _) => {
                                    output_by_symbol(&response, &outputs.ids(), data.process_data)
                                        .ok_or_else(|| {
                                        Error::MissingOutput(
                                            gateway.to_string(),
                                            name_or_id.to_string(),
                                        )
                                    })?
                                }
                                With::Default => return Err(cannot_use_default(gateway)),
                                With::Flow(_) => return Err(cannot_use_cond_expr(gateway)),
                                With::Fork(_) => return Err(cannot_fork(gateway)),
                            }
                        }
                        (GatewayType::Parallel, _) => {
                            return match inputs.len() {
                                1 => Ok(Return::Fork(outputs.ids())),
                                _ => Ok(Return::JoinFork(outputs.ids())),
                            };
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
        }
        Ok(Return::End(None))
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

pub(super) type ExecuteResult<'a> = Result<Option<&'a Bpmn>, Error>;

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
