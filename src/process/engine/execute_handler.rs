use log::debug;

use crate::model::Gateway;
use std::borrow::Cow;

#[derive(Default, Debug)]
pub(super) struct ExecuteHandler<'a> {
    data: Vec<Cow<'a, [usize]>>,
    uncommitted: Vec<Cow<'a, [usize]>>,
    token_stack: Vec<TokenData<'a>>,
}

impl<'a> ExecuteHandler<'a> {
    pub(super) fn new(item: Cow<'a, [usize]>) -> Self {
        Self {
            data: vec![item],
            uncommitted: Default::default(),
            token_stack: Default::default(),
        }
    }

    // Take everything to be processed.
    pub(super) fn take(&mut self) -> Vec<Cow<'a, [usize]>> {
        std::mem::take(&mut self.data)
    }

    // Push directly to data without the involvement of token_stack.
    // When we join a gateway with one output we should not increase the token_stack.
    pub(super) fn immediate(&mut self, item: Cow<'a, [usize]>) {
        self.data.push(item);
    }

    // New tokens cannot be added immediately until all processed tokens have been correlated.
    pub(super) fn pending(&mut self, item: Cow<'a, [usize]>) {
        self.uncommitted.push(item);
    }

    // Commit all new tokens.
    pub(super) fn commit(&mut self) {
        for item in std::mem::take(&mut self.uncommitted) {
            debug!("NEW TOKENS {}", item.len());
            self.token_stack.push(TokenData::new(item.len()));
            self.data.push(item);
        }
    }

    // Consume a token. Might be a gateway join or end event.
    pub(super) fn consume_token(&mut self, join: Option<&'a Gateway>) {
        if let Some(token_data) = self.token_stack.last_mut() {
            token_data.consume(join);
        }
    }

    // Once all tokens have been consumed, return the gateways involved.
    pub(super) fn tokens_consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        if let Some(token_data) = self.token_stack.last_mut()
            && token_data.consumed()
        {
            debug!(
                "ALL CONSUMED created: {}, consumed: {}",
                token_data.created, token_data.consumed
            );

            #[cfg(debug_assertions)]
            return self.token_stack.pop().map(|data| data.joined).map(dedup);
            #[cfg(not(debug_assertions))]
            return self.token_stack.pop().map(|data| data.joined);
        }
        None
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

#[cfg(debug_assertions)]
fn dedup(mut input: Vec<&Gateway>) -> Vec<&Gateway> {
    let mut seen = std::collections::HashSet::new();
    input.retain(|v| seen.insert(*v.id.local()));

    // If many different gateways are visited, we have an unbalanced graph
    if input.len() > 1 {
        log::error!("Unbalanced diagram detected!");
    }
    input
}
