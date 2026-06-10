use crate::{
    Error,
    bpmn::{Gateway, GatewayType},
};
use log::debug;
use std::{borrow::Cow, fmt::Display};

#[derive(Default, Debug)]
pub(super) struct ExecuteHandler<'a> {
    tokens_ready: Vec<Cow<'a, [usize]>>,
    uncommitted: Vec<Cow<'a, [usize]>>,
    token_stack: Vec<TokenData<'a>>,
}

impl<'a> ExecuteHandler<'a> {
    pub(super) fn new(tokens: Cow<'a, [usize]>) -> Self {
        Self {
            tokens_ready: vec![tokens],
            uncommitted: Default::default(),
            token_stack: Default::default(),
        }
    }

    // Return tokens to be processed.
    pub(super) fn active_tokens(&mut self) -> Vec<Cow<'a, [usize]>> {
        std::mem::take(&mut self.tokens_ready)
    }

    // Push directly to tokens_ready without the involvement of token_stack.
    // When we JOIN a gateway with one output we should not increase the token_stack.
    pub(super) fn immediate(&mut self, item: Cow<'a, [usize]>) {
        self.tokens_ready.push(item);
    }

    // If a gateway FORK is involved, we need to use the token stack. Even if the gateway only selects one flow.
    pub(super) fn pending_fork(&mut self, item: Cow<'a, [usize]>) {
        self.uncommitted.push(item);
    }

    // Commit all new tokens.
    pub(super) fn commit(&mut self) {
        for item in self.uncommitted.drain(..) {
            debug!("NEW TOKENS {}", item.len());
            self.token_stack.push(TokenData::new(item.len()));
            self.tokens_ready.push(item);
        }
    }

    // Consume a token. Might be a gateway join or end event.
    pub(super) fn consume_token(&mut self, join: Option<&'a Gateway>) {
        if let Some(token_data) = self.token_stack.last_mut() {
            token_data.consume(join);
        }
    }

    // Once all tokens have been consumed, return the gateway involved.
    pub(super) fn tokens_consumed(&mut self) -> Result<Option<&'a Gateway>, Error> {
        if let Some(mut token_data) = self.token_stack.pop_if(|token_data| token_data.consumed())
            && let tokens_arrived = token_data.joined.len()
        {
            debug!("ALL CONSUMED {}", token_data);

            #[cfg(debug_assertions)]
            check_unbalanced_diagram(&token_data.joined)?;

            let gateway = token_data.joined.pop();

            // Determines whether enough tokens have arrived at the parallel gateway.
            // Without this, parallel gateways are too permissive.
            if let Some(
                gateway @ Gateway {
                    gateway_type: GatewayType::Parallel,
                    inputs,
                    ..
                },
            ) = gateway
                && tokens_arrived < *inputs as usize
            {
                return Err(Error::BpmnRequirement(format!(
                    "Execution stopped. Not enough tokens at {gateway}"
                )));
            }
            return Ok(gateway);
        }
        Ok(None)
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

    fn consumed(&self) -> bool {
        self.created.saturating_sub(self.consumed) == 0
    }
}

impl<'a> Display for TokenData<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "created: {}, consumed: {}, joined: {}",
            self.created,
            self.consumed,
            self.joined
                .iter()
                .map(|gw| gw.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[cfg(debug_assertions)]
fn check_unbalanced_diagram(input: &[&Gateway]) -> Result<(), Error> {
    // If many different gateways are visited, we have an unbalanced graph
    if std::collections::HashSet::<usize>::from_iter(input.iter().map(|g| *g.id.local())).len() > 1
    {
        return Err(Error::NotSupported("Unbalanced diagram".into()));
    }
    Ok(())
}
