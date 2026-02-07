use crate::{
    Error,
    api::{Data, IntermediateEvent, TaskResult, With},
    error::FUNC_MAP_ERROR_MSG,
};
use std::{collections::HashMap, fmt::Display};

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

    pub(super) fn run_task(&self, index: usize, data: Data<T>) -> Result<TaskResult, Error> {
        if let Some(Callback::Task(func)) = self.callbacks.get(index) {
            Ok(func(data))
        } else {
            Err(Error::MissingImplementation(format!(
                "Task with index: {index}"
            )))
        }
    }

    pub(super) fn run_exclusive(
        &self,
        index: usize,
        data: Data<T>,
    ) -> Result<Option<&'static str>, Error> {
        if let Some(Callback::Exclusive(func)) = self.callbacks.get(index) {
            Ok(func(data))
        } else {
            Err(Error::MissingImplementation(format!(
                "Exclusive with index: {index}"
            )))
        }
    }

    pub(super) fn run_inclusive(&self, index: usize, data: Data<T>) -> Result<With, Error> {
        if let Some(Callback::Inclusive(func)) = self.callbacks.get(index) {
            Ok(func(data))
        } else {
            Err(Error::MissingImplementation(format!(
                "Inclusive with index: {index}"
            )))
        }
    }

    pub(super) fn run_eventbased(
        &self,
        index: usize,
        data: Data<T>,
    ) -> Result<IntermediateEvent, Error> {
        if let Some(Callback::EventBased(func)) = self.callbacks.get(index) {
            Ok(func(data))
        } else {
            Err(Error::MissingImplementation(format!(
                "Eventbased with index: {index}"
            )))
        }
    }

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
