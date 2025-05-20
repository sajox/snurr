use crate::{
    IntermediateEvent, With,
    model::{Boundary, GatewayType},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Option<Boundary>;

type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync>;
type ExclusiveCallback<T> = Box<dyn Fn(Data<T>) -> Option<&'static str> + Sync>;
type InclusiveCallback<T> = Box<dyn Fn(Data<T>) -> With + Sync>;
type EventBasedCallback<T> = Box<dyn Fn(Data<T>) -> IntermediateEvent + Sync>;

pub struct Handler<T> {
    task: Vec<TaskCallback<T>>,
    exclusive: Vec<ExclusiveCallback<T>>,
    inclusive: Vec<InclusiveCallback<T>>,
    event_based: Vec<EventBasedCallback<T>>,
    handler_map: HandlerMap,
}

#[derive(Default, Debug)]
pub struct HandlerMap {
    pub task: HashMap<String, usize>,
    pub exclusive: HashMap<String, usize>,
    pub inclusive: HashMap<String, usize>,
    pub event_based: HashMap<String, usize>,
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self {
            task: Default::default(),
            exclusive: Default::default(),
            inclusive: Default::default(),
            event_based: Default::default(),
            handler_map: Default::default(),
        }
    }
}

impl<T> Handler<T> {
    pub(super) fn add_task<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        let name = name.into();
        if self
            .handler_map
            .task
            .insert(name.clone(), self.task.len())
            .is_some()
        {
            log::warn!(r#"Installed Task with name "{}" multiple times"#, name);
        }
        self.task.push(Box::new(func));
    }

    pub(super) fn add_exclusive<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync,
    {
        let name = name.into();
        if self
            .handler_map
            .exclusive
            .insert(name.clone(), self.exclusive.len())
            .is_some()
        {
            log::warn!(
                r#"Installed {} with name "{}" multiple times"#,
                GatewayType::Exclusive,
                name
            );
        }
        self.exclusive.push(Box::new(func));
    }

    pub(super) fn add_inclusive<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        let name = name.into();
        if self
            .handler_map
            .inclusive
            .insert(name.clone(), self.inclusive.len())
            .is_some()
        {
            log::warn!(
                r#"Installed {} with name "{}" multiple times"#,
                GatewayType::Inclusive,
                name
            );
        }
        self.inclusive.push(Box::new(func));
    }

    pub(super) fn add_event_based<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> IntermediateEvent + 'static + Sync,
    {
        let name = name.into();
        if self
            .handler_map
            .event_based
            .insert(name.clone(), self.event_based.len())
            .is_some()
        {
            log::warn!(
                r#"Installed {} with name "{}" multiple times"#,
                GatewayType::EventBased,
                name
            );
        }
        self.event_based.push(Box::new(func));
    }

    pub(super) fn take_handler_map(&mut self) -> HandlerMap {
        std::mem::take(&mut self.handler_map)
    }

    pub(super) fn run_task(&self, index: usize, data: Data<T>) -> Option<TaskResult> {
        self.task.get(index).map(|value| (*value)(data))
    }

    pub(super) fn run_exclusive(
        &self,
        index: usize,
        data: Data<T>,
    ) -> Option<Option<&'static str>> {
        self.exclusive.get(index).map(|value| (*value)(data))
    }

    pub(super) fn run_inclusive(&self, index: usize, data: Data<T>) -> Option<With> {
        self.inclusive.get(index).map(|value| (*value)(data))
    }

    pub(super) fn run_event_based(&self, index: usize, data: Data<T>) -> Option<IntermediateEvent> {
        self.event_based.get(index).map(|value| (*value)(data))
    }
}
