use crate::bpmn::Symbol;
use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Option<Boundary>;

/// Inclusive gateway return type
#[derive(Default, Debug)]
pub enum With {
    #[default]
    Default,
    /// Outgoing sequence flow by name or id
    Flow(&'static str),
    /// Collection of outgoing sequence flow by name or id
    Fork(Vec<&'static str>),
}

impl From<&'static str> for With {
    fn from(value: &'static str) -> Self {
        Self::Flow(value)
    }
}

impl From<Vec<&'static str>> for With {
    fn from(value: Vec<&'static str>) -> Self {
        Self::Fork(value)
    }
}

/// Task return type
#[derive(Debug)]
pub enum Boundary {
    Symbol(Symbol),
    NameSymbol(&'static str, Symbol),
}

impl Boundary {
    pub(crate) fn symbol(&self) -> &Symbol {
        match self {
            Boundary::Symbol(symbol) | Boundary::NameSymbol(_, symbol) => symbol,
        }
    }

    pub(crate) fn name(&self) -> Option<&'static str> {
        match self {
            Boundary::NameSymbol(name, _) => Some(name),
            _ => None,
        }
    }
}

impl From<(&'static str, Symbol)> for Boundary {
    fn from(value: (&'static str, Symbol)) -> Self {
        Self::NameSymbol(value.0, value.1)
    }
}

impl From<Symbol> for Boundary {
    fn from(symbol: Symbol) -> Self {
        Self::Symbol(symbol)
    }
}

impl Display for Boundary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Boundary::Symbol(symbol) => write!(f, "{symbol}"),
            Boundary::NameSymbol(name, symbol) => write!(f, "({name}, {symbol})"),
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
