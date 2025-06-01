use log::debug;

use crate::model::Gateway;
use std::borrow::Cow;

#[derive(Default, Debug)]
pub(super) struct BpmnQueue<'a> {
    queue: Vec<Cow<'a, [usize]>>,
    uncommitted: Vec<Cow<'a, [usize]>>,
    token_handler: TokenHandler<'a>,
}

impl<'a> BpmnQueue<'a> {
    pub(super) fn new(item: Cow<'a, [usize]>) -> Self {
        Self {
            queue: vec![item],
            uncommitted: Default::default(),
            token_handler: Default::default(),
        }
    }

    pub(super) fn take(&mut self) -> Vec<Cow<'a, [usize]>> {
        std::mem::take(&mut self.queue)
    }

    pub(super) fn push_output(&mut self, item: Cow<'a, [usize]>) {
        self.queue.push(item);
    }

    pub(super) fn add_pending(&mut self, item: Cow<'a, [usize]>) {
        self.uncommitted.push(item);
    }

    pub(super) fn commit(&mut self) {
        for item in std::mem::take(&mut self.uncommitted) {
            self.token_handler.push(item.len());
            self.queue.push(item);
        }
    }

    pub(super) fn join_token(&mut self, gateway: &'a Gateway) {
        self.token_handler.consume(Some(gateway))
    }

    pub(super) fn end_token(&mut self) {
        self.token_handler.consume(None);
    }

    pub(super) fn tokens_consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        self.token_handler.consumed()
    }
}

#[derive(Default, Debug)]
struct TokenData<'a> {
    created: usize,
    joined: Vec<&'a Gateway>,
    consumed: usize,
}

impl<'a> TokenData<'a> {
    fn new(created: usize) -> Self {
        Self {
            created,
            joined: Default::default(),
            consumed: Default::default(),
        }
    }

    fn consume(&mut self, maybe_gateway: Option<&'a Gateway>) {
        if let Some(gateway) = maybe_gateway {
            self.joined.push(gateway)
        }
        self.consumed += 1;
        debug!("TOKENS CONSUMED {}", self.consumed);
    }

    fn consumed(&mut self) -> bool {
        self.created.saturating_sub(self.consumed) == 0
    }
}

// Keep track of tokens alive. For both parallel and inclusive.
// Only inclusive adds joined gateways due to different behavioral mechanisms.
#[derive(Default, Debug)]
struct TokenHandler<'a> {
    stack: Vec<TokenData<'a>>,
}

impl<'a> TokenHandler<'a> {
    fn consume(&mut self, join: Option<&'a Gateway>) {
        if let Some(token_data) = self.stack.last_mut() {
            token_data.consume(join);
        }
    }

    // Once all tokens have been used up, return the gateways involved.
    fn consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        if let Some(token_data) = self.stack.last_mut() {
            if token_data.consumed() {
                debug!(
                    "ALL CONSUMED created: {}, consumed: {}",
                    token_data.created, token_data.consumed
                );

                #[cfg(debug_assertions)]
                return self.stack.pop().map(|data| data.joined).map(dedup);
                #[cfg(not(debug_assertions))]
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

#[cfg(debug_assertions)]
fn dedup(mut input: Vec<&Gateway>) -> Vec<&Gateway> {
    let mut seen = std::collections::HashSet::new();
    input.retain(|v| seen.insert(*v.id.local()));
    input
}
