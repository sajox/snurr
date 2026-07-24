use crate::{
    Error,
    api::{Exclusive, Inclusive, IntermediateEvent, Task},
    error::FUNC_MAP_ERROR_MSG,
};
use std::{collections::HashMap, fmt::Display};

macro_rules! callback {
    ($name:ident, $variant:pat => $value:ident, $ret:ty) => {
        pub(super) fn $name(&self, index: usize, data: &T) -> Result<$ret, Error> {
            let Some($variant) = self.callbacks.get(index) else {
                return Err(Error::MissingImplementation(format!(
                    "{} with index: {index}",
                    stringify!($name)
                )));
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

    // Used while building. Is None after use.
    handler_map: Option<HandlerMap>,
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self {
            callbacks: Default::default(),
            handler_map: Some(Default::default()),
        }
    }
}

impl<T> Handler<T> {
    pub(super) fn add_callback(&mut self, name: impl Into<String>, callback: Callback<T>) {
        if let Some(hm) = &mut self.handler_map {
            hm.insert(
                match callback {
                    Callback::Task(_) => HandlerType::Task,
                    Callback::Exclusive(_) => HandlerType::Exclusive,
                    Callback::Inclusive(_) => HandlerType::Inclusive,
                    Callback::EventBased(_) => HandlerType::EventBased,
                },
                name,
                self.callbacks.len(),
            );
            self.callbacks.push(callback);
        }
    }

    callback!(run_task, Callback::Task(func) => func, Task);
    callback!(run_exclusive, Callback::Exclusive(func) => func, Exclusive);
    callback!(run_inclusive, Callback::Inclusive(func) => func, Inclusive);
    callback!(run_eventbased, Callback::EventBased(func) => func, IntermediateEvent);

    // Consumes the handler_map and cannot add more things with add_
    pub(super) fn build(&mut self) -> Result<HandlerMap, Error> {
        self.handler_map
            .take()
            .ok_or_else(|| Error::Builder(FUNC_MAP_ERROR_MSG.into()))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum HandlerType {
    Task,
    Exclusive,
    Inclusive,
    EventBased,
}

impl Display for HandlerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Default, Debug)]
pub struct HandlerMap {
    map: HashMap<HandlerType, HashMap<String, usize>>,
}

impl HandlerMap {
    pub fn get(&self, handler_type: HandlerType, key: &str) -> Option<&usize> {
        if let Some(inner_map) = self.map.get(&handler_type) {
            inner_map.get(key)
        } else {
            None
        }
    }

    fn insert(&mut self, handler_type: HandlerType, name: impl Into<String>, index: usize) {
        let name = name.into();
        if self
            .map
            .entry(handler_type)
            .or_default()
            .insert(name.clone(), index)
            .is_some()
        {
            log::warn!(r#"Installed {handler_type} with name "{name}" multiple times"#);
        }
    }
}
