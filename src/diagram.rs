mod events;
pub mod reader;

use crate::{
    bpmn::{Activity, ActivityType, Bpmn, BpmnType, Event, Gateway, GatewayType, Symbol},
    diagram::events::Events,
    error::RuntimeError,
    process::func_map::FuncMap,
};

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

#[derive(Debug)]
pub struct Diagram {
    process_index: usize,
    data: Box<[ProcessData]>,
}

impl Diagram {
    fn new(process_index: usize, data: Box<[ProcessData]>) -> Self {
        Self {
            process_index,
            data,
        }
    }

    pub fn main_process(&self) -> Result<&ProcessData, RuntimeError> {
        self.get_process(self.process_index)
    }

    // Can be a process or sub process
    pub fn get_process(&self, process_id: usize) -> Result<&ProcessData, RuntimeError> {
        self.data.get(process_id).ok_or_else(|| {
            RuntimeError::Engine(format!("missing process data with index `{}`", process_id))
        })
    }

    pub fn data(&self) -> &[ProcessData] {
        &self.data
    }

    pub fn install_and_check(&mut self, func_map: &FuncMap) -> HashSet<String> {
        let mut missing = HashSet::new();
        for process_data in self.data.iter_mut() {
            for bpmn in &mut process_data.data {
                match bpmn {
                    Bpmn::Activity(Activity {
                        id,
                        name,
                        func_idx,
                        activity_type,
                        ..
                    }) if !matches!(activity_type, ActivityType::SubProcess) => {
                        if let Some(id) = func_map.get_id(BpmnType::Task, id, name.as_deref()) {
                            func_idx.replace(id);
                        } else {
                            missing.insert(format!(
                                "{activity_type}: {}",
                                name.as_deref().unwrap_or(id.bpmn())
                            ));
                        }
                    }
                    Bpmn::Gateway(Gateway {
                        gateway_type:
                            gateway_type @ (GatewayType::EventBased
                            | GatewayType::Exclusive
                            | GatewayType::Inclusive),
                        name,
                        id,
                        func_idx,
                        outputs,
                        ..
                    }) if outputs.len() > 1 => {
                        if let Some(id) =
                            func_map.get_id((*gateway_type).into(), id, name.as_deref())
                        {
                            func_idx.replace(id);
                        } else {
                            missing.insert(format!(
                                "{gateway_type}: {}",
                                name.as_deref().unwrap_or(id.bpmn())
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        missing
    }
}

#[derive(Default, Debug)]
pub struct ProcessData {
    // Start event in the process
    start: usize,
    data: Box<[Bpmn]>,
    pub events: Events,
}

impl ProcessData {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn get(&self, index: usize) -> Option<&Bpmn> {
        self.data.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bpmn> {
        self.data.iter()
    }

    pub fn find_by_name_or_id<'a>(&self, search: &str, outputs: &'a Outputs) -> Option<&'a usize> {
        outputs.iter().find(|index| {
            if let Some(Bpmn::SequenceFlow { id, name, .. }) = self.get(**index) {
                return name.as_deref().is_some_and(|name| name == search) || id.bpmn() == search;
            }
            false
        })
    }

    pub fn find_by_intermediate_event<'a>(
        &self,
        name: &str,
        symbol: Symbol,
        outputs: &'a Outputs,
    ) -> Option<&'a usize> {
        outputs.iter().find(|index| {
            if let Some(Bpmn::SequenceFlow { target_ref, .. }) = self.get(**index)
                && let Some(bpmn) = self.get(*target_ref.local())
            {
                return match bpmn {
                    // We can target both ReceiveTask or Events.
                    Bpmn::Activity(Activity {
                        activity_type: ActivityType::ReceiveTask,
                        name: Some(name_check),
                        ..
                    }) => symbol == Symbol::Message && name_check.as_str() == name,
                    Bpmn::Event(Event {
                        symbol:
                            symbol_check @ (Symbol::Message
                            | Symbol::Signal
                            | Symbol::Timer
                            | Symbol::Conditional),

                        name: Some(name_check),
                        ..
                    }) => symbol_check == &symbol && name_check.as_str() == name,
                    _ => false,
                };
            }
            false
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct Outputs {
    bpmn_ids: Box<[String]>,
    local_ids: Box<[usize]>,
}

impl Display for Outputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bpmn_ids.join(", "))
    }
}

impl Outputs {
    fn new(bpmn_ids: Vec<String>) -> Outputs {
        let len = bpmn_ids.len();
        Self {
            bpmn_ids: bpmn_ids.into_boxed_slice(),
            local_ids: vec![0; len].into_boxed_slice(),
        }
    }

    pub(crate) fn ids(&self) -> &[usize] {
        &self.local_ids
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &usize> {
        self.local_ids.iter()
    }

    pub(crate) fn len(&self) -> usize {
        self.local_ids.len()
    }

    pub(crate) fn first(&self) -> Option<&usize> {
        self.local_ids.first()
    }

    fn update_local_ids(&mut self, bpmn_index: &HashMap<String, usize>) {
        for (idx, value) in self.bpmn_ids.iter().enumerate() {
            if let Some(index) = bpmn_index.get(value) {
                self.local_ids[idx] = *index;
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Id {
    bpmn_id: String,
    local_id: usize,
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {})", self.bpmn_id, self.local_id)
    }
}

impl Id {
    pub(crate) fn bpmn(&self) -> &str {
        &self.bpmn_id
    }

    pub(crate) fn local(&self) -> &usize {
        &self.local_id
    }

    fn update_local_id(&mut self, map: &HashMap<String, usize>) {
        if let Some(index) = map.get(&self.bpmn_id) {
            self.local_id = *index;
        }
    }
}

impl From<String> for Id {
    fn from(bpmn_id: String) -> Self {
        Self {
            bpmn_id,
            local_id: 0,
        }
    }
}

impl Bpmn {
    fn id(&self) -> &str {
        match self {
            Bpmn::Event(Event { id, .. })
            | Bpmn::SequenceFlow { id, .. }
            | Bpmn::Activity(Activity { id, .. })
            | Bpmn::Gateway(Gateway { id, .. }) => id.bpmn(),
        }
    }

    fn update_local_id(&mut self, value: usize) {
        match self {
            Bpmn::Event(Event { id, .. })
            | Bpmn::SequenceFlow { id, .. }
            | Bpmn::Activity(Activity { id, .. })
            | Bpmn::Gateway(Gateway { id, .. }) => id.local_id = value,
        }
    }
}
