use log::debug;

use crate::model::{Gateway, GatewayType, HashMap};
use std::borrow::Cow;

#[derive(Default, Debug)]
pub struct BpmnQueue<'a> {
    queue: Vec<Cow<'a, [usize]>>,
    uncommitted: Vec<Cow<'a, [usize]>>,
    inclusive_handler: InclusiveGatewayHandler<'a>,
    parallel_handler: ParallelGatewayHandler,
}

impl<'a> BpmnQueue<'a> {
    pub fn new(item: Cow<'a, [usize]>) -> Self {
        Self {
            queue: vec![item],
            uncommitted: Default::default(),
            inclusive_handler: Default::default(),
            parallel_handler: Default::default(),
        }
    }

    pub fn take(&mut self) -> Vec<Cow<'a, [usize]>> {
        std::mem::take(&mut self.queue)
    }

    pub fn push_output(&mut self, item: Cow<'a, [usize]>) {
        self.queue.push(item);
    }

    pub fn push_fork(&mut self, item: Cow<'a, [usize]>) {
        self.inclusive_handler.push(item.len());
        self.queue.push(item);
    }

    pub fn add_pending(&mut self, item: Cow<'a, [usize]>) {
        self.uncommitted.push(item);
    }

    pub fn commit(&mut self) {
        for item in std::mem::take(&mut self.uncommitted) {
            self.push_fork(item);
        }
    }

    pub fn join_token(&mut self, gateway: &'a Gateway) {
        match gateway.gateway {
            GatewayType::Parallel => {
                if let Some(gw) = self.parallel_handler.consume(gateway) {
                    self.push_output(Cow::Borrowed(gw.outputs.ids()));
                }
            }
            GatewayType::Inclusive => self.inclusive_handler.consume(Some(gateway)),
            _ => {}
        }
    }

    pub fn end_token(&mut self) {
        self.inclusive_handler.consume(None);
    }

    pub fn tokens_consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        self.inclusive_handler.consumed()
    }
}

#[derive(Default, Debug)]
struct TokenData<'a> {
    created: usize,
    joined: Vec<&'a Gateway>,
    consumed: usize,
}

impl TokenData<'_> {
    fn new(created: usize) -> Self {
        Self {
            created,
            joined: Default::default(),
            consumed: Default::default(),
        }
    }
}

#[derive(Default, Debug)]
struct InclusiveGatewayHandler<'a> {
    stack: Vec<TokenData<'a>>,
}

impl<'a> InclusiveGatewayHandler<'a> {
    fn consume(&mut self, join: Option<&'a Gateway>) {
        if let Some(TokenData {
            joined, consumed, ..
        }) = self.stack.last_mut()
        {
            if let Some(gateway) = join {
                joined.push(gateway)
            }
            *consumed += 1;
            debug!("TOKENS CONSUMED {}", consumed);
        }
    }

    // Once all tokens have been used up, return the gateways involved.
    fn consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        if let Some(TokenData {
            created, consumed, ..
        }) = self.stack.last_mut()
        {
            if created.saturating_sub(*consumed) == 0 {
                debug!("ALL CONSUMED expected: {}, consumed: {}", created, consumed);
                return self.stack.pop().map(|data| data.joined);
            }
        }
        None
    }

    fn push(&mut self, tokens: usize) {
        debug!("NEW TOKENS {}", tokens);
        self.stack.push(TokenData::new(tokens));
    }
}

// Keeps the state of the visited parallel gateways.
#[derive(Debug, Default)]
struct ParallelGatewayHandler {
    state: HashMap<usize, usize>,
}

impl ParallelGatewayHandler {
    fn consume<'a>(&mut self, join: &'a Gateway) -> Option<&'a Gateway> {
        let consumed = self.state.entry(*join.id.local()).or_default();
        *consumed += 1;
        debug!("TOKENS CONSUMED {}", consumed);
        if *consumed >= join.inputs.len() {
            debug!(
                "ALL CONSUMED expected: {}, consumed: {}",
                join.inputs.len(),
                consumed
            );
            *consumed -= join.inputs.len();
            return Some(join);
        }
        None
    }
}
