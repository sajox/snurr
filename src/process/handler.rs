use crate::{
    api::{Exclusive, Inclusive, IntermediateEvent, Task},
    process::{RuntimeError},
};

macro_rules! callback {
    ($name:ident, $variant:pat => $value:ident, $ret:ty) => {
        pub(super) fn $name(&self, index: usize, data: &T) -> Result<$ret, RuntimeError> {
            let Some($variant) = self.callbacks.get(index) else {
                Err(RuntimeError::Engine(format!(
                    "missing {} with index: {index}",
                    stringify!($name)
                )))?
            };
            Ok($value(data))
        }
    };
}

type TaskCallback<T> = Box<dyn Fn(&T) -> Task + Sync + Send>;
type ExclusiveCallback<T> = Box<dyn Fn(&T) -> Exclusive + Sync + Send>;
type InclusiveCallback<T> = Box<dyn Fn(&T) -> Inclusive + Sync + Send>;
type EventBasedCallback<T> = Box<dyn Fn(&T) -> IntermediateEvent + Sync + Send>;

pub(super) enum Callback<T> {
    Task(TaskCallback<T>),
    Exclusive(ExclusiveCallback<T>),
    Inclusive(InclusiveCallback<T>),
    EventBased(EventBasedCallback<T>),
}

pub(super) struct Handler<T> {
    callbacks: Vec<Callback<T>>,
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self {
            callbacks: Default::default(),
        }
    }
}

impl<T> Handler<T> {
    // add callback and return index
    pub(super) fn add_callback(&mut self, callback: Callback<T>) -> usize {
        let index = self.callbacks.len();
        self.callbacks.push(callback);
        index
    }

    callback!(run_task, Callback::Task(func) => func, Task);
    callback!(run_exclusive, Callback::Exclusive(func) => func, Exclusive);
    callback!(run_inclusive, Callback::Inclusive(func) => func, Inclusive);
    callback!(run_eventbased, Callback::EventBased(func) => func, IntermediateEvent);
}
