use crate::{Error, IntermediateEvent, With, error::FUNC_MAP_ERROR_MSG, model::Boundary};
use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Option<Boundary>;

type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync + Send>;
type ExclusiveCallback<T> = Box<dyn Fn(Data<T>) -> Option<&'static str> + Sync + Send>;
type InclusiveCallback<T> = Box<dyn Fn(Data<T>) -> With + Sync + Send>;
type EventBasedCallback<T> = Box<dyn Fn(Data<T>) -> IntermediateEvent + Sync + Send>;

pub(super) enum Callback<T> {
    Task(TaskCallback<T>),
    Exclusive(ExclusiveCallback<T>),
    Inclusive(InclusiveCallback<T>),
    EventBased(EventBasedCallback<T>),
}

pub(super) enum CallbackResult {
    Task(TaskResult),
    Exclusive(Option<&'static str>),
    Inclusive(With),
    EventBased(IntermediateEvent),
}

impl<T> Callback<T> {
    fn run(&self, data: Data<T>) -> CallbackResult {
        match self {
            Callback::Task(func) => CallbackResult::Task((*func)(data)),
            Callback::Exclusive(func) => CallbackResult::Exclusive((*func)(data)),
            Callback::Inclusive(func) => CallbackResult::Inclusive((*func)(data)),
            Callback::EventBased(func) => CallbackResult::EventBased((*func)(data)),
        }
    }
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

    // Consumes the handler_map and cannot add more things with add_
    pub(super) fn build(&mut self) -> Result<HandlerMap, Error> {
        self.handler_map
            .take()
            .ok_or_else(|| Error::Builder(FUNC_MAP_ERROR_MSG.into()))
    }

    pub(super) fn run(&self, index: usize, data: Data<T>) -> Option<CallbackResult> {
        self.callbacks.get(index).map(|cb| cb.run(data))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(super) enum HandlerType {
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
pub(super) struct HandlerMap {
    map: HashMap<HandlerType, HashMap<String, usize>>,
}

impl HandlerMap {
    pub(super) fn get(&self, handler_type: HandlerType, key: &str) -> Option<&usize> {
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
            warn(handler_type, name);
        }
    }
}

fn warn(ty: impl Display, name: impl Display) {
    log::warn!(r#"Installed {ty} with name "{name}" multiple times"#);
}
