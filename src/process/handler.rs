use crate::{
    api::{Exclusive, Inclusive, IntermediateEvent, Task},
    bpmn::GatewayType,
    diagram::Id,
    process::{RuntimeError, RuntimeErrorKind},
};
use std::{borrow::Cow, collections::HashMap, fmt::Display};

macro_rules! callback {
    ($name:ident, $variant:pat => $value:ident, $ret:ty) => {
        pub(super) fn $name(&self, index: usize, data: &T) -> Result<$ret, RuntimeError> {
            let Some($variant) = self.callbacks.get(index) else {
                Err(RuntimeErrorKind::Engine(format!(
                    "missing {} with index: {index}",
                    stringify!($name)
                )))?
            };
            Ok($value(data))
        }
    };
}

type TaskCallback<T> = Box<dyn Fn(&T) -> Task + Sync + Send>;
type ExclusiveCallback<T> = Box<dyn Fn(&T) -> Exclusive + Sync + Send>;
type InclusiveCallback<T> = Box<dyn Fn(&T) -> Inclusive + Sync + Send>;
type EventBasedCallback<T> = Box<dyn Fn(&T) -> IntermediateEvent + Sync + Send>;

pub(super) enum Callback<T> {
    Task(TaskCallback<T>),
    Exclusive(ExclusiveCallback<T>),
    Inclusive(InclusiveCallback<T>),
    EventBased(EventBasedCallback<T>),
}

pub(super) struct Handler<T> {
    callbacks: Vec<Callback<T>>,

    // Temporary function mapping that associates a type and a name with an array ID.
    func_map: Option<FuncMap>,
}

impl<T> Default for Handler<T> {
    fn default() -> Self {
        Self {
            callbacks: Default::default(),
            func_map: Some(Default::default()),
        }
    }
}

impl<T> Handler<T> {
    pub(super) fn add_callback(&mut self, name: impl Into<String>, callback: Callback<T>) {
        if let Some(fm) = &mut self.func_map {
            fm.insert(
                match callback {
                    Callback::Task(_) => FuncType::Task,
                    Callback::Exclusive(_) => FuncType::Exclusive,
                    Callback::Inclusive(_) => FuncType::Inclusive,
                    Callback::EventBased(_) => FuncType::EventBased,
                },
                name.into(),
                self.callbacks.len(),
            );
            self.callbacks.push(callback);
        }
    }

    callback!(run_task, Callback::Task(func) => func, Task);
    callback!(run_exclusive, Callback::Exclusive(func) => func, Exclusive);
    callback!(run_inclusive, Callback::Inclusive(func) => func, Inclusive);
    callback!(run_eventbased, Callback::EventBased(func) => func, IntermediateEvent);

    // consumes the func_map and cannot add more things with add_
    pub(super) fn finished(&mut self) -> Option<FuncMap> {
        self.func_map.take()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum FuncType {
    Task,
    Exclusive,
    Inclusive,
    Parallel,
    EventBased,
}

impl From<GatewayType> for FuncType {
    fn from(value: GatewayType) -> Self {
        match value {
            GatewayType::Exclusive => FuncType::Exclusive,
            GatewayType::Inclusive => FuncType::Inclusive,
            GatewayType::Parallel => FuncType::Parallel,
            GatewayType::EventBased => FuncType::EventBased,
        }
    }
}

impl Display for FuncType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

#[derive(Default, Debug)]
pub struct FuncMap {
    // Use `Cow` to avoid creating an owned `String` when comparing.
    map: HashMap<(FuncType, Cow<'static, str>), usize>,
}

impl FuncMap {
    // Check if bpmn id or name is registered by user. Begin with bpmn id as it is unique and
    // then try with the name if it exist.
    pub fn get_id(&self, ty: FuncType, id: &Id, name: Option<&str>) -> Option<usize> {
        [Some(id.bpmn()), name]
            .into_iter()
            .flatten()
            .find_map(|s| self.map.get(&(ty, Cow::Borrowed(s))))
            .copied()
    }

    fn insert(&mut self, ty: FuncType, name: String, index: usize) {
        if self
            .map
            .insert((ty, Cow::Owned(name.clone())), index)
            .is_some()
        {
            log::warn!(r#"Installed {ty} with name "{name}" multiple times"#);
        }
    }
}
