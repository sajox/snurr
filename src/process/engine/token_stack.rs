#[derive(Default, Debug)]
pub struct TokenStack {
    queue: Vec<usize>,
}

impl TokenStack {
    // When all tokens has been consumed i.e reached 0, then remove it and return true. False otherwise.
    pub fn remove_token(&mut self) -> bool {
        if let Some(value) = self.queue.last_mut() {
            *value = value.saturating_sub(1);
            if *value == 0 {
                self.queue.pop();
                return true;
            }
        }
        false
    }

    pub fn push(&mut self, tokens: usize) {
        self.queue.push(tokens);
    }
}
