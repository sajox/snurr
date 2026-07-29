use crate::bpmn::Symbol;

/// Inclusive gateway return type
#[derive(Default, Debug)]
pub enum Inclusive {
    /// Use default flow
    #[default]
    Default,
    /// Outgoing sequence flow by name or id
    Flow(&'static str),
    /// Collection of outgoing sequence flow by name or id. An empty Vec selects the default sequence flow.
    Fork(Vec<&'static str>),
    /// Terminate process early and return specified error
    Panic(Box<dyn std::error::Error + Send + Sync>),
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
    /// Terminate process early and return specified error
    Panic(Box<dyn std::error::Error + Send + Sync>),
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
    /// Terminate process early and return specified error
    Panic(Box<dyn std::error::Error + Send + Sync>),
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

/// Event based gateway return type
#[derive(Debug)]
pub enum IntermediateEvent {
    /// Throw intermediate event to correlate to matching catch
    Throw(&'static str, Symbol),
    /// Terminate process early and return specified error
    Panic(Box<dyn std::error::Error + Send + Sync>),
}

impl From<(&'static str, Symbol)> for IntermediateEvent {
    fn from(value: (&'static str, Symbol)) -> Self {
        Self::Throw(value.0, value.1)
    }
}
