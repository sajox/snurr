use crate::{
    Error, IntermediateEvent, With,
    model::{ActivityType, Boundary, GatewayType},
};
use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

const FUNC_MAP_ERROR_MSG: &str = "couldn't fetch function map";

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Option<Boundary>;

type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync>;
type ExclusiveCallback<T> = Box<dyn Fn(Data<T>) -> Option<&'static str> + Sync>;
type InclusiveCallback<T> = Box<dyn Fn(Data<T>) -> With + Sync>;
type EventBasedCallback<T> = Box<dyn Fn(Data<T>) -> IntermediateEvent + Sync>;

#[derive(Default, Debug)]
pub(super) struct HandlerMap {
    pub task: HashMap<String, usize>,
    pub exclusive: HashMap<String, usize>,
    pub inclusive: HashMap<String, usize>,
    pub event_based: HashMap<String, usize>,
}

pub(super) struct Handler<T> {
    task: Vec<TaskCallback<T>>,
    exclusive: Vec<ExclusiveCallback<T>>,
    inclusive: Vec<InclusiveCallback<T>>,
    event_based: Vec<EventBasedCallback<T>>,

    // Used while building. Is None after use.
    handler_map: Option<HandlerMap>,
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self {
            task: Default::default(),
            exclusive: Default::default(),
            inclusive: Default::default(),
            event_based: Default::default(),
            handler_map: Some(Default::default()),
        }
    }
}

impl<T> Handler<T> {
    pub(super) fn add_task<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        let Some(hm) = &mut self.handler_map else {
            return;
        };

        let name = name.into();
        if hm.task.insert(name.clone(), self.task.len()).is_some() {
            Self::warn(ActivityType::Task, name);
        }
        self.task.push(Box::new(func));
    }

    pub(super) fn add_exclusive<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync,
    {
        let Some(hm) = &mut self.handler_map else {
            return;
        };

        let name = name.into();
        if hm
            .exclusive
            .insert(name.clone(), self.exclusive.len())
            .is_some()
        {
            Self::warn(GatewayType::Exclusive, name);
        }
        self.exclusive.push(Box::new(func));
    }

    pub(super) fn add_inclusive<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        let Some(hm) = &mut self.handler_map else {
            return;
        };

        let name = name.into();
        if hm
            .inclusive
            .insert(name.clone(), self.inclusive.len())
            .is_some()
        {
            Self::warn(GatewayType::Inclusive, name);
        }
        self.inclusive.push(Box::new(func));
    }

    pub(super) fn add_event_based<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> IntermediateEvent + 'static + Sync,
    {
        let Some(hm) = &mut self.handler_map else {
            return;
        };

        let name = name.into();
        if hm
            .event_based
            .insert(name.clone(), self.event_based.len())
            .is_some()
        {
            Self::warn(GatewayType::EventBased, name);
        }
        self.event_based.push(Box::new(func));
    }

    fn warn(ty: impl Display, name: impl Display) {
        log::warn!(r#"Installed {ty} with name "{name}" multiple times"#);
    }

    // Consumes the handler_map and cannot add more things with add_
    pub(super) fn build(&mut self) -> Result<HandlerMap, Error> {
        self.handler_map
            .take()
            .ok_or_else(|| Error::Builder(FUNC_MAP_ERROR_MSG.into()))
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
