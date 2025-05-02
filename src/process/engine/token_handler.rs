use crate::model::Gateway;

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
pub struct TokenHandler<'a> {
    stack: Vec<Data<'a>>,
}

impl<'a> TokenHandler<'a> {
    // When all tokens has been consumed with join and end then return the gateways involved.
    pub fn join(&mut self, gateway: &'a Gateway) -> Option<Vec<&'a Gateway>> {
        if let Some(Data {
            tokens,
            gateways: gateway_ids,
            ends,
        }) = self.stack.last_mut()
        {
            gateway_ids.push(gateway);
            if tokens.saturating_sub(gateway_ids.len() + *ends) == 0 {
                return self.stack.pop().map(|data| data.gateways);
            }
        }
        None
    }

    pub fn end(&mut self) -> Option<Vec<&'a Gateway>> {
        if let Some(Data {
            tokens,
            gateways: gateway_ids,
            ends,
        }) = self.stack.last_mut()
        {
            *ends += 1;
            if tokens.saturating_sub(gateway_ids.len() + *ends) == 0 {
                return self.stack.pop().map(|data| data.gateways);
            }
        }
        None
    }

    pub fn push(&mut self, tokens: usize) {
        self.stack.push(Data::new(tokens));
    }
}
