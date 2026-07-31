use crate::bpmn::Symbol;
use std::borrow::Cow;

/// Inclusive gateway return type
#[derive(Default, Debug)]
pub enum Inclusive {
    /// Use default flow
    #[default]
    Default,
    /// Outgoing sequence flow by name or id
    Flow(Cow<'static, str>),
    /// Collection of outgoing sequence flow by name or id. An empty Vec selects the default sequence flow.
    Fork(Vec<Cow<'static, str>>),
    /// Terminate the process prematurely and have it return the specified error.
    /// Instead of doing this, you should ensure that the BPMN diagram is always modeled
    /// with an error path whenever possible.
    Panic(Box<dyn std::error::Error + Send + Sync>),
}

/// Convenient factory methods
impl Inclusive {
    pub fn flow<S>(value: S) -> Inclusive
    where
        S: Into<Cow<'static, str>>,
    {
        Self::Flow(value.into())
    }

    pub fn fork<S>(value: S) -> Inclusive
    where
        S: IntoIterator<Item: Into<Cow<'static, str>>>,
    {
        Self::Fork(value.into_iter().map(Into::into).collect())
    }

    pub fn panic<S>(value: S) -> Inclusive
    where
        S: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::Panic(value.into())
    }
}

impl From<&'static str> for Inclusive {
    fn from(value: &'static str) -> Self {
        Self::Flow(value.into())
    }
}

impl From<String> for Inclusive {
    fn from(value: String) -> Self {
        Self::Flow(value.into())
    }
}

impl From<Cow<'static, str>> for Inclusive {
    fn from(value: Cow<'static, str>) -> Self {
        Self::Flow(value)
    }
}

impl From<Vec<&'static str>> for Inclusive {
    fn from(value: Vec<&'static str>) -> Self {
        Self::Fork(value.into_iter().map(Into::into).collect())
    }
}

impl From<Vec<String>> for Inclusive {
    fn from(value: Vec<String>) -> Self {
        Self::Fork(value.into_iter().map(Into::into).collect())
    }
}

impl From<Vec<Cow<'static, str>>> for Inclusive {
    fn from(value: Vec<Cow<'static, str>>) -> Self {
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
    Flow(Cow<'static, str>),
    /// Terminate the process prematurely and have it return the specified error.
    /// Instead of doing this, you should ensure that the BPMN diagram is always modeled
    /// with an error path whenever possible.
    Panic(Box<dyn std::error::Error + Send + Sync>),
}

/// Convenient factory methods
impl Exclusive {
    pub fn flow<S>(value: S) -> Exclusive
    where
        S: Into<Cow<'static, str>>,
    {
        Self::Flow(value.into())
    }

    pub fn panic<S>(value: S) -> Exclusive
    where
        S: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::Panic(value.into())
    }
}

impl<S> From<S> for Exclusive
where
    S: Into<Cow<'static, str>>,
{
    fn from(value: S) -> Self {
        Self::Flow(value.into())
    }
}

/// Task return type
#[derive(Default, Debug)]
pub enum Task {
    /// Use default flow
    #[default]
    Default,
    /// Use a task boundary with optional name and a symbol
    Boundary(Option<Cow<'static, str>>, Symbol),
    /// Terminate the process prematurely and have it return the specified error.
    /// Instead of doing this, you should ensure that the BPMN diagram is always modeled
    /// with an error path whenever possible.
    Panic(Box<dyn std::error::Error + Send + Sync>),
}

/// Convenient factory methods
impl Task {
    pub fn boundary<S>(name: Option<S>, symbol: Symbol) -> Task
    where
        S: Into<Cow<'static, str>>,
    {
        match name {
            Some(value) => Self::Boundary(Some(value.into()), symbol),
            None => Self::Boundary(None, symbol),
        }
    }

    pub fn panic<S>(value: S) -> Task
    where
        S: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::Panic(value.into())
    }
}

impl<S> From<(S, Symbol)> for Task
where
    S: Into<Cow<'static, str>>,
{
    fn from(value: (S, Symbol)) -> Self {
        Self::Boundary(Some(value.0.into()), value.1)
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
    Throw(Cow<'static, str>, Symbol),
    /// Terminate the process prematurely and have it return the specified error.
    /// Instead of doing this, you should ensure that the BPMN diagram is always modeled
    /// with an error path whenever possible.
    Panic(Box<dyn std::error::Error + Send + Sync>),
}

/// Convenient factory methods
impl IntermediateEvent {
    pub fn throw<S>(name: S, symbol: Symbol) -> IntermediateEvent
    where
        S: Into<Cow<'static, str>>,
    {
        Self::Throw(name.into(), symbol)
    }

    pub fn panic<S>(value: S) -> IntermediateEvent
    where
        S: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::Panic(value.into())
    }
}

impl<S> From<(S, Symbol)> for IntermediateEvent
where
    S: Into<Cow<'static, str>>,
{
    fn from(value: (S, Symbol)) -> Self {
        Self::Throw(value.0.into(), value.1)
    }
}
