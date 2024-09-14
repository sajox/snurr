use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use log::warn;

use crate::Symbol;

// Messages
const MISSING_FUNCTION: &str = "Missing function. Please register";

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Result<(), Symbol>;

/// Task callback that use `Data` type as input and return a result containing a success unit type ()
/// or a failure that contains a `Symbol` for an alternate flow.
type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync>;

/// Gateway callback that use `Data` type as input and return a `Vec` with flow(s) to take
type GatewayCallback<T> = Box<dyn Fn(Data<T>) -> Vec<&'static str> + Sync>;

/// Event handler to add task or gateway closures by name or id
pub struct Eventhandler<T> {
    task_func: HashMap<String, TaskCallback<T>>,
    gateway_func: HashMap<String, GatewayCallback<T>>,
}

impl<T> Default for Eventhandler<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Eventhandler<T> {
    /// Create new empty event handler
    pub fn new() -> Self {
        Self {
            task_func: Default::default(),
            gateway_func: Default::default(),
        }
    }

    /// Add a task to the event handler by name or id with corresponding closure.
    /// ```
    /// use snurr::Eventhandler;
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// let mut handler: Eventhandler<Counter> = Eventhandler::default();
    /// handler.add_task("Count 1", |input| {
    ///     input.lock().unwrap().count += 1;
    ///     Ok(())
    /// });
    /// ```
    pub fn add_task<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        self.task_func.insert(name.into(), Box::new(func));
    }

    pub(crate) fn run_task(&self, key: &str, data: Data<T>) -> TaskResult {
        if let Some(func) = self.task_func.get(key) {
            return (*func)(data);
        } else {
            warn!("{}: {}", MISSING_FUNCTION, key);
        }
        Ok(())
    }

    /// Add a gateway to the event handler by name or id with corresponding closure.
    /// ```
    /// use snurr::Eventhandler;
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// let mut handler: Eventhandler<Counter> = Eventhandler::default();
    /// handler.add_gateway("equal to 3", |input| {
    ///     let result = if input.lock().unwrap().count == 3 {
    ///         "YES"
    ///     } else {
    ///         "NO"
    ///     };
    ///     vec![result]
    /// });
    /// ```
    pub fn add_gateway<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> Vec<&'static str> + 'static + Sync,
    {
        self.gateway_func.insert(name.into(), Box::new(func));
    }

    pub(crate) fn run_gateway(&self, key: &str, data: Data<T>) -> Vec<&'static str> {
        if let Some(func) = self.gateway_func.get(key) {
            return (*func)(data);
        }
        warn!("{}: {}", MISSING_FUNCTION, key);
        vec![]
    }
}
