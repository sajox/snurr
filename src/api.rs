use crate::bpmn::Symbol;
use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Inclusive gateway return type
#[derive(Default, Debug)]
pub enum Inclusive {
    /// Use default flow
    #[default]
    Default,
    /// Outgoing sequence flow by name or id
    Flow(&'static str),
    /// Collection of outgoing sequence flow by name or id
    Fork(Vec<&'static str>),
}

impl From<&'static str> for Inclusive {
    fn from(value: &'static str) -> Self {
        Self::Flow(value)
    }
}

impl From<Vec<&'static str>> for Inclusive {
    fn from(value: Vec<&'static str>) -> Self {
        Self::Fork(value)
    }
}

/// Exclusive gateway return type
#[derive(Default, Debug)]
pub enum Exclusive {
    /// Use default flow
    #[default]
    Default,
    /// Outgoing sequence flow by name or id
    Flow(&'static str),
}

impl From<&'static str> for Exclusive {
    fn from(value: &'static str) -> Self {
        Self::Flow(value)
    }
}

/// Task return type
#[derive(Default, Debug)]
pub enum Task {
    /// Use default flow
    #[default]
    Default,
    /// Use a task boundary with optional name and a symbol
    Boundary(Option<&'static str>, Symbol),
}

impl From<(&'static str, Symbol)> for Task {
    fn from(value: (&'static str, Symbol)) -> Self {
        Self::Boundary(Some(value.0), value.1)
    }
}

impl From<Symbol> for Task {
    fn from(symbol: Symbol) -> Self {
        Self::Boundary(None, symbol)
    }
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::Default => write!(f, "Default"),
            Task::Boundary(name, symbol) => {
                write!(f, "({}, {symbol})", name.unwrap_or_default())
            }
        }
    }
}

/// Event based gateway return type
#[derive(Debug)]
pub struct IntermediateEvent(pub &'static str, pub Symbol);

impl From<(&'static str, Symbol)> for IntermediateEvent {
    fn from(value: (&'static str, Symbol)) -> Self {
        Self(value.0, value.1)
    }
}

impl Display for IntermediateEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}
