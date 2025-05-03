use crate::model::Gateway;
use std::borrow::Cow;

#[derive(Default, Debug)]
pub struct BpmnQueue<'a> {
    queue: Vec<Cow<'a, [usize]>>,
    token_handler: TokenHandler<'a>,
}

impl<'a> BpmnQueue<'a> {
    pub fn new(item: Cow<'a, [usize]>) -> Self {
        Self {
            queue: vec![item],
            token_handler: Default::default(),
        }
    }

    pub fn pop(&mut self) -> Option<Cow<'a, [usize]>> {
        self.queue.pop()
    }

    pub fn push(&mut self, item: Cow<'a, [usize]>) {
        self.token_handler.push(item.len());
        self.queue.push(item);
    }

    pub fn is_empty(&mut self) -> bool {
        self.queue.is_empty()
    }

    pub fn join_token(&mut self, gateway: &'a Gateway) {
        self.token_handler.join(gateway);
    }

    pub fn end_token(&mut self) {
        self.token_handler.end();
    }

    pub fn tokens_consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        self.token_handler.consumed()
    }
}

#[derive(Default, Debug)]
struct Data<'a> {
    tokens: usize,
    gateways: Vec<&'a Gateway>,
    ends: usize,
}

impl Data<'_> {
    fn new(tokens: usize) -> Self {
        Self {
            tokens,
            gateways: Default::default(),
            ends: Default::default(),
        }
    }
}

#[derive(Default, Debug)]
struct TokenHandler<'a> {
    stack: Vec<Data<'a>>,
}

impl<'a> TokenHandler<'a> {
    // When all tokens has been consumed with join and end then return the gateways involved.
    fn join(&mut self, gateway: &'a Gateway) {
        if let Some(Data { gateways, .. }) = self.stack.last_mut() {
            gateways.push(gateway);
        }
    }

    fn end(&mut self) {
        if let Some(Data { ends, .. }) = self.stack.last_mut() {
            *ends += 1;
        }
    }

    fn consumed(&mut self) -> Option<Vec<&'a Gateway>> {
        if let Some(Data {
            gateways,
            tokens,
            ends,
        }) = self.stack.last_mut()
        {
            if tokens.saturating_sub(gateways.len() + *ends) == 0 {
                return self.stack.pop().map(|data| data.gateways);
            }
        }
        None
    }

    fn push(&mut self, tokens: usize) {
        self.stack.push(Data::new(tokens));
    }
}
