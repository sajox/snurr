use crate::{Boundary, Symbol, With};
use std::sync::{Arc, Mutex};

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Option<Boundary>;

type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync>;
type ExclusiveCallback<T> = Box<dyn Fn(Data<T>) -> Option<&'static str> + Sync>;
type InclusiveCallback<T> = Box<dyn Fn(Data<T>) -> With + Sync>;
type EventBasedCallback<T> = Box<dyn Fn(Data<T>) -> (Option<&'static str>, Symbol) + Sync>;

pub struct Handler<T> {
    task: Vec<TaskCallback<T>>,
    exclusive: Vec<ExclusiveCallback<T>>,
    inclusive: Vec<InclusiveCallback<T>>,
    event_based: Vec<EventBasedCallback<T>>,
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self {
            task: Default::default(),
            exclusive: Default::default(),
            inclusive: Default::default(),
            event_based: Default::default(),
        }
    }
}

impl<T> Handler<T> {
    pub fn add_task<F>(&mut self, func: F) -> usize
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        let len = self.task.len();
        self.task.push(Box::new(func));
        len
    }

    pub fn add_exclusive<F>(&mut self, func: F) -> usize
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync,
    {
        let len = self.exclusive.len();
        self.exclusive.push(Box::new(func));
        len
    }

    pub fn add_inclusive<F>(&mut self, func: F) -> usize
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        let len = self.inclusive.len();
        self.inclusive.push(Box::new(func));
        len
    }

    pub fn add_event_based<F>(&mut self, func: F) -> usize
    where
        F: Fn(Data<T>) -> (Option<&'static str>, Symbol) + 'static + Sync,
    {
        let len = self.event_based.len();
        self.event_based.push(Box::new(func));
        len
    }

    pub(crate) fn run_task(&self, index: usize, data: Data<T>) -> Option<TaskResult> {
        self.task.get(index).map(|value| (*value)(data))
    }

    pub(crate) fn run_exclusive(
        &self,
        index: usize,
        data: Data<T>,
    ) -> Option<Option<&'static str>> {
        self.exclusive.get(index).map(|value| (*value)(data))
    }

    pub(crate) fn run_inclusive(&self, index: usize, data: Data<T>) -> Option<With> {
        self.inclusive.get(index).map(|value| (*value)(data))
    }

    pub(crate) fn run_event_based(
        &self,
        index: usize,
        data: Data<T>,
    ) -> Option<(Option<&'static str>, Symbol)> {
        self.event_based.get(index).map(|value| (*value)(data))
    }
}
