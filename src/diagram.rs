mod events;
pub mod reader;

use crate::{
    bpmn::{Activity, ActivityType, Bpmn, Event, EventType, Gateway, GatewayType, Symbol},
    diagram::events::Events,
    process::{
        DiagramErrorKind, ParseError, ParseErrorKind, RuntimeError,
        handler::{HandlerMap, HandlerType},
    },
};

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::AddAssign,
};

#[derive(Debug)]
pub struct Diagram {
    data: Vec<ProcessData>,
}

impl Diagram {
    fn new(data: Vec<ProcessData>) -> Self {
        Self { data }
    }

    // All top level processes defined in Definitions.
    // Always last in the Vec as it is a top level construct in the XML.
    pub fn get_definition(&self) -> Option<&ProcessData> {
        self.data.last()
    }

    // Can be a process or sub process
    pub fn get_process(&self, process_id: usize) -> Option<&ProcessData> {
        self.data.get(process_id)
    }

    pub fn data(&self) -> &[ProcessData] {
        self.data.as_slice()
    }

    pub fn install_and_check(&mut self, handler_map: HandlerMap) -> HashSet<String> {
        let mut missing = HashSet::new();
        for process_data in self.data.iter_mut() {
            for bpmn in &mut process_data.data {
                match bpmn {
                    Bpmn::Activity(Activity {
                        id,
                        name,
                        func_idx,
                        activity_type:
                            activity_type @ (ActivityType::Task
                            | ActivityType::ScriptTask
                            | ActivityType::UserTask
                            | ActivityType::ServiceTask
                            | ActivityType::CallActivity
                            | ActivityType::ReceiveTask
                            | ActivityType::SendTask
                            | ActivityType::ManualTask
                            | ActivityType::BusinessRuleTask),
                        ..
                    }) => {
                        if let Some(id) =
                            get_func_id(id, name.as_deref(), HandlerType::Task, &handler_map)
                        {
                            func_idx.replace(*id);
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
                        let handler_type = match gateway_type {
                            GatewayType::Exclusive => HandlerType::Exclusive,
                            GatewayType::Inclusive => HandlerType::Inclusive,
                            GatewayType::EventBased => HandlerType::EventBased,
                            _ => continue,
                        };
                        if let Some(id) =
                            get_func_id(id, name.as_deref(), handler_type, &handler_map)
                        {
                            func_idx.replace(*id);
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

// Check if bpmn id or name should be installed. Begin with bpmn id as it is unique and
// then try with the name if it exist.
fn get_func_id<'a>(
    id: &Id,
    name: Option<&str>,
    handler_type: HandlerType,
    handler_map: &'a HandlerMap,
) -> Option<&'a usize> {
    [Some(id.bpmn()), name]
        .into_iter()
        .flatten()
        .find_map(|s| handler_map.get(handler_type, s))
}

#[derive(Default, Debug)]
pub struct ProcessData {
    // Start event in the process
    start: Option<usize>,
    data: Vec<Bpmn>,
    events: Events,
}

impl ProcessData {
    fn add(&mut self, mut bpmn: Bpmn) -> Result<(), ParseError> {
        let len = self.data.len();
        if let Bpmn::Event(Event {
            event_type: EventType::Start,
            symbol: None,
            ..
        }) = bpmn
            && self.start.replace(len).is_some()
        {
            Err(ParseErrorKind::NotSupported("multiple start event".into()))?
        }

        bpmn.update_local_id(len);
        self.data.push(bpmn);
        Ok(())
    }

    // Everything in the process has been collected. Update local IDs with correct index.
    fn finalize(&mut self) {
        // Collect Bpmn id to index in array
        let bpmn_index: HashMap<String, usize> = self
            .data
            .iter()
            .enumerate()
            .filter_map(|(index, bpmn)| bpmn.id().map(|id| (id.into(), index)))
            .collect();

        self.data.iter_mut().for_each(|bpmn| match bpmn {
            Bpmn::Activity(Activity { outputs, .. }) => outputs.update_local_ids(&bpmn_index),
            Bpmn::Event(event) => {
                event.outputs.update_local_ids(&bpmn_index);
                if let Some(attached_to_ref) = &mut event.attached_to_ref {
                    attached_to_ref.update_local_id(&bpmn_index);
                }

                self.events.register(event);
            }
            Bpmn::Gateway(Gateway {
                default, outputs, ..
            }) => {
                outputs.update_local_ids(&bpmn_index);
                if let Some(default) = default {
                    default.update_local_id(&bpmn_index)
                }
            }
            Bpmn::SequenceFlow { target_ref, .. } => target_ref.update_local_id(&bpmn_index),
            _ => {}
        });
    }

    pub fn start(&self) -> Result<usize, RuntimeError> {
        self.start.ok_or(DiagramErrorKind::MissingStartEvent.into())
    }

    pub fn get(&self, index: usize) -> Option<&Bpmn> {
        self.data.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bpmn> {
        self.data.iter()
    }

    pub fn events(&self) -> &Events {
        &self.events
    }

    pub fn find_boundary<'a>(
        &'a self,
        activity_id: &Id,
        search_name: Option<&str>,
        search_symbol: &Symbol,
    ) -> Option<&'a usize> {
        self.events
            .boundaries(activity_id)?
            .iter()
            .filter_map(|index| self.data.get(*index))
            .find_map(|bpmn| match bpmn {
                Bpmn::Event(Event {
                    symbol: Some(symbol),
                    id,
                    name,
                    ..
                }) if symbol == search_symbol && search_name == name.as_deref() => Some(id.local()),
                _ => None,
            })
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
                            Some(
                                symbol_check @ (Symbol::Message
                                | Symbol::Signal
                                | Symbol::Timer
                                | Symbol::Conditional),
                            ),
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
    bpmn_ids: Vec<String>,
    local_ids: Vec<usize>,
}

impl Display for Outputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bpmn_ids.join(", "))
    }
}

impl Outputs {
    fn add(&mut self, output_id: impl Into<String>) {
        self.bpmn_ids.push(output_id.into());
        self.local_ids.push(0);
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
    fn id(&self) -> Option<&str> {
        match self {
            Bpmn::Event(Event { id, .. })
            | Bpmn::SequenceFlow { id, .. }
            | Bpmn::Activity(Activity { id, .. })
            | Bpmn::Definitions { id, .. }
            | Bpmn::Gateway(Gateway { id, .. })
            | Bpmn::Process { id, .. } => Some(id.bpmn()),
            _ => None,
        }
    }

    fn update_local_id(&mut self, value: usize) {
        match self {
            Bpmn::Event(Event { id, .. })
            | Bpmn::SequenceFlow { id, .. }
            | Bpmn::Activity(Activity { id, .. })
            | Bpmn::Definitions { id, .. }
            | Bpmn::Gateway(Gateway { id, .. })
            | Bpmn::Process { id, .. } => id.local_id = value,
            _ => {}
        }
    }

    fn update_data_index(&mut self, value: usize) {
        match self {
            Bpmn::Activity(Activity {
                activity_type: ActivityType::SubProcess { data_index },
                ..
            })
            | Bpmn::Process { data_index, .. } => {
                data_index.replace(value);
            }
            _ => {}
        }
    }

    fn add_output(&mut self, text: String) {
        match self {
            Bpmn::Event(Event { outputs, .. })
            | Bpmn::Gateway(Gateway { outputs, .. })
            | Bpmn::Activity(Activity { outputs, .. }) => outputs.add(text),
            _ => {}
        }
    }

    fn add_input(&mut self) {
        if let Bpmn::Gateway(Gateway { inputs, .. }) = self {
            inputs.add_assign(1);
        }
    }
}
