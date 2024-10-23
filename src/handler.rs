use crate::model::{Boundary, HashMap, With};
use log::warn;
use std::sync::{Arc, Mutex};

// Messages
const MISSING_FUNCTION: &str = "Missing function. Please register";

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Option<Boundary>;

/// Task callback that use `Data` type as input and return a None for regular flow
/// or Some(Boundary) for an alternate flow.
type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync>;

/// Gateway callback that use `Data` type as input and return a `With` enum.
type GatewayCallback<T> = Box<dyn Fn(Data<T>) -> With + Sync>;

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
    ///     None
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
        None
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
    ///     result.into()
    /// });
    /// ```
    pub fn add_gateway<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        self.gateway_func.insert(name.into(), Box::new(func));
    }

    pub(crate) fn run_gateway(&self, key: &str, data: Data<T>) -> With {
        if let Some(func) = self.gateway_func.get(key) {
            return (*func)(data);
        }
        warn!("{}: {}", MISSING_FUNCTION, key);
        With::Default
    }
}
