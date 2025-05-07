use crate::model::Gateway;
use std::borrow::Cow;

#[derive(Default, Debug)]
pub struct BpmnQueue<'a> {
    queue: Vec<Cow<'a, [usize]>>,
    token_handler: TokenHandler<'a>,
    uncommitted: Vec<Cow<'a, [usize]>>,
}

impl<'a> BpmnQueue<'a> {
    pub fn new(item: Cow<'a, [usize]>) -> Self {
        Self {
            queue: vec![item],
            token_handler: Default::default(),
            uncommitted: Default::default(),
        }
    }

    pub fn pop(&mut self) -> Option<Cow<'a, [usize]>> {
        self.queue.pop()
    }

    pub fn push(&mut self, item: Cow<'a, [usize]>) {
        self.token_handler.push(item.len());
        self.queue.push(item);
    }

    pub fn add_pending_fork(&mut self, item: Cow<'a, [usize]>) {
        self.uncommitted.push(item);
    }

    pub fn commit_forks(&mut self) {
        for item in std::mem::take(&mut self.uncommitted) {
            self.push(item);
        }
    }

    pub fn join_token(&mut self, gateway: &'a Gateway) {
        self.token_handler.consume_token(Some(gateway));
    }

    pub fn end_token(&mut self) {
        self.token_handler.consume_token(None);
    }

    pub fn tokens_consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        self.token_handler.consumed()
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
struct TokenHandler<'a> {
    stack: Vec<TokenData<'a>>,
}

impl<'a> TokenHandler<'a> {
    fn consume_token(&mut self, join: Option<&'a Gateway>) {
        if let Some(TokenData {
            joined, consumed, ..
        }) = self.stack.last_mut()
        {
            if let Some(gateway) = join {
                joined.push(gateway)
            }
            *consumed += 1;
        }
    }

    // Once all tokens have been used up, return the gateways involved.
    fn consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        if let Some(TokenData {
            created, consumed, ..
        }) = self.stack.last_mut()
        {
            if created.saturating_sub(*consumed) == 0 {
                return self.stack.pop().map(|data| data.joined);
            }
        }
        None
    }

    fn push(&mut self, tokens: usize) {
        self.stack.push(TokenData::new(tokens));
    }
}
