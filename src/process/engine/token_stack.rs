#[derive(Default, Debug)]
pub struct TokenStack {
    stack: Vec<usize>,
}

impl TokenStack {
    // When all tokens has been consumed i.e reached 0, then remove it and return true. False otherwise.
    pub fn remove(&mut self, value: usize) -> bool {
        if let Some(current) = self.stack.last_mut() {
            *current = current.saturating_sub(value);
            if *current == 0 {
                self.stack.pop();
                return true;
            }
        }
        false
    }

    pub fn push(&mut self, tokens: usize) {
        self.stack.push(tokens);
    }
}
