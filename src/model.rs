use crate::error::Error;
use core::fmt;
use std::{collections::HashMap, fmt::Display};

pub(crate) const DEFINITIONS: &[u8] = b"definitions";
pub(crate) const PROCESS: &[u8] = b"process";

// Event
pub(crate) const START_EVENT: &[u8] = b"startEvent";
pub(crate) const END_EVENT: &[u8] = b"endEvent";
pub(crate) const BOUNDARY_EVENT: &[u8] = b"boundaryEvent";
pub(crate) const INTERMEDIATE_CATCH_EVENT: &[u8] = b"intermediateCatchEvent";
pub(crate) const INTERMEDIATE_THROW_EVENT: &[u8] = b"intermediateThrowEvent";

// Event symbol
pub(crate) const CANCEL_EVENT_DEFINITION: &[u8] = b"cancelEventDefinition";
pub(crate) const COMPENSATE_EVENT_DEFINITION: &[u8] = b"compensateEventDefinition";
pub(crate) const CONDITIONAL_EVENT_DEFINITION: &[u8] = b"conditionalEventDefinition";
pub(crate) const ERROR_EVENT_DEFINITION: &[u8] = b"errorEventDefinition";
pub(crate) const ESCALATION_EVENT_DEFINITION: &[u8] = b"escalationEventDefinition";
pub(crate) const MESSAGE_EVENT_DEFINITION: &[u8] = b"messageEventDefinition";
pub(crate) const LINK_EVENT_DEFINITION: &[u8] = b"linkEventDefinition";
pub(crate) const SIGNAL_EVENT_DEFINITION: &[u8] = b"signalEventDefinition";
pub(crate) const TERMINATE_EVENT_DEFINITION: &[u8] = b"terminateEventDefinition";
pub(crate) const TIMER_EVENT_DEFINITION: &[u8] = b"timerEventDefinition";

// Task
pub(crate) const TASK: &[u8] = b"task";
pub(crate) const SERVICE_TASK: &[u8] = b"serviceTask";
pub(crate) const USER_TASK: &[u8] = b"userTask";
pub(crate) const SCRIPT_TASK: &[u8] = b"scriptTask";
pub(crate) const RECEIVE_TASK: &[u8] = b"receiveTask";
pub(crate) const SEND_TASK: &[u8] = b"sendTask";
pub(crate) const MANUAL_TASK: &[u8] = b"manualTask";
pub(crate) const BUSINESS_RULE_TASK: &[u8] = b"businessRuleTask";
pub(crate) const CALL_ACTIVITY: &[u8] = b"callActivity";
pub(crate) const SUB_PROCESS: &[u8] = b"subProcess";
pub(crate) const TRANSACTION: &[u8] = b"transaction";

// Direction
pub(crate) const OUTGOING: &[u8] = b"outgoing";
pub(crate) const INCOMING: &[u8] = b"incoming";

// Flow
pub(crate) const SEQUENCE_FLOW: &[u8] = b"sequenceFlow";

// Gateway
pub(crate) const EXCLUSIVE_GATEWAY: &[u8] = b"exclusiveGateway";
pub(crate) const PARALLEL_GATEWAY: &[u8] = b"parallelGateway";
pub(crate) const INCLUSIVE_GATEWAY: &[u8] = b"inclusiveGateway";
pub(crate) const EVENT_BASED_GATEWAY: &[u8] = b"eventBasedGateway";

// Attributes
pub(crate) const ATTRIB_ID: &[u8] = b"id";
pub(crate) const ATTRIB_IS_EXECUTABLE: &[u8] = b"isExecutable";
pub(crate) const ATTRIB_NAME: &[u8] = b"name";
pub(crate) const ATTRIB_SOURCE_REF: &[u8] = b"sourceRef";
pub(crate) const ATTRIB_TARGET_REF: &[u8] = b"targetRef";
pub(crate) const ATTRIB_DEFAULT: &[u8] = b"default";
pub(crate) const _ATTRIB_EXPORTER_VERSION: &[u8] = b"exporterVersion";
pub(crate) const ATTRIB_ATTACHED_TO_REF: &[u8] = b"attachedToRef";
pub(crate) const ATTRIB_CANCEL_ACTIVITY: &[u8] = b"cancelActivity";

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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum EventType {
    Boundary,
    End,
    IntermediateCatch,
    IntermediateThrow,
    Start,
}

impl TryFrom<&[u8]> for EventType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            BOUNDARY_EVENT => EventType::Boundary,
            END_EVENT => EventType::End,
            INTERMEDIATE_CATCH_EVENT => EventType::IntermediateCatch,
            INTERMEDIATE_THROW_EVENT => EventType::IntermediateThrow,
            START_EVENT => EventType::Start,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ));
            }
        })
    }
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum ActivityType {
    SubProcess { data_index: Option<usize> },
    Task,
    ScriptTask,
    UserTask,
    ServiceTask,
    CallActivity,
    ReceiveTask,
    SendTask,
    ManualTask,
    BusinessRuleTask,
}

impl TryFrom<&[u8]> for ActivityType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            SUB_PROCESS | TRANSACTION => ActivityType::SubProcess { data_index: None },
            TASK => ActivityType::Task,
            SCRIPT_TASK => ActivityType::ScriptTask,
            USER_TASK => ActivityType::UserTask,
            SERVICE_TASK => ActivityType::ServiceTask,
            CALL_ACTIVITY => ActivityType::CallActivity,
            RECEIVE_TASK => ActivityType::ReceiveTask,
            SEND_TASK => ActivityType::SendTask,
            MANUAL_TASK => ActivityType::ManualTask,
            BUSINESS_RULE_TASK => ActivityType::BusinessRuleTask,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ));
            }
        })
    }
}

impl Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum GatewayType {
    Exclusive,
    Inclusive,
    Parallel,
    EventBased,
}

impl TryFrom<&[u8]> for GatewayType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            EXCLUSIVE_GATEWAY => GatewayType::Exclusive,
            INCLUSIVE_GATEWAY => GatewayType::Inclusive,
            PARALLEL_GATEWAY => GatewayType::Parallel,
            EVENT_BASED_GATEWAY => GatewayType::EventBased,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ));
            }
        })
    }
}

impl Display for GatewayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum DirectionType {
    Incoming,
    Outgoing,
}

impl TryFrom<&[u8]> for DirectionType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            INCOMING => DirectionType::Incoming,
            OUTGOING => DirectionType::Outgoing,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ));
            }
        })
    }
}

impl Display for DirectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

/// BPMN Symbols
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Symbol {
    Cancel,
    Compensation,
    Conditional,
    Error,
    Escalation,
    Link,
    Message,
    Signal,
    Terminate,
    Timer,
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl TryFrom<&[u8]> for Symbol {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Error> {
        let ty = match value {
            CANCEL_EVENT_DEFINITION => Symbol::Cancel,
            COMPENSATE_EVENT_DEFINITION => Symbol::Compensation,
            CONDITIONAL_EVENT_DEFINITION => Symbol::Conditional,
            ERROR_EVENT_DEFINITION => Symbol::Error,
            ESCALATION_EVENT_DEFINITION => Symbol::Escalation,
            MESSAGE_EVENT_DEFINITION => Symbol::Message,
            LINK_EVENT_DEFINITION => Symbol::Link,
            SIGNAL_EVENT_DEFINITION => Symbol::Signal,
            TERMINATE_EVENT_DEFINITION => Symbol::Terminate,
            TIMER_EVENT_DEFINITION => Symbol::Timer,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ));
            }
        };
        Ok(ty)
    }
}

#[derive(Debug)]
pub(crate) struct Gateway {
    pub(crate) gateway_type: GatewayType,
    pub(crate) id: Id,
    pub(crate) func_idx: Option<usize>,
    pub(crate) name: Option<String>,
    pub(crate) default: Option<Id>,
    pub(crate) outputs: Outputs,
    pub(crate) inputs: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct Event {
    pub(crate) event_type: EventType,
    pub(crate) symbol: Option<Symbol>,
    pub(crate) id: Id,
    pub(crate) name: Option<String>,
    pub(crate) attached_to_ref: Option<Id>,
    pub(crate) _cancel_activity: Option<String>,
    pub(crate) outputs: Outputs,
}

#[derive(Debug)]
pub(crate) enum Bpmn {
    Activity {
        activity_type: ActivityType,
        id: Id,
        func_idx: Option<usize>,
        name: Option<String>,
        outputs: Outputs,
    },
    Definitions {
        id: Id,
    },
    Direction {
        direction_type: DirectionType,
        text: Option<String>,
    },
    Event(Event),
    Gateway(Gateway),
    Process {
        id: Id,
        data_index: Option<usize>,
        _is_executable: bool,
    },
    SequenceFlow {
        id: Id,
        name: Option<String>,
        _source_ref: String,
        target_ref: Id,
    },
}

impl Bpmn {
    pub(crate) fn id(&self) -> Result<&Id, Error> {
        match self {
            Bpmn::Event(Event { id, .. })
            | Bpmn::SequenceFlow { id, .. }
            | Bpmn::Activity { id, .. }
            | Bpmn::Definitions { id, .. }
            | Bpmn::Gateway(Gateway { id, .. })
            | Bpmn::Process { id, .. } => Ok(id),
            Bpmn::Direction { direction_type, .. } => {
                Err(Error::MissingId(direction_type.to_string()))
            }
        }
    }

    pub(crate) fn update_data_index(&mut self, value: usize) {
        match self {
            Bpmn::Activity {
                activity_type: ActivityType::SubProcess { data_index },
                ..
            }
            | Bpmn::Process { data_index, .. } => {
                data_index.replace(value);
            }
            _ => {}
        }
    }

    pub(crate) fn update_local_id(&mut self, value: usize) {
        match self {
            Bpmn::Event(Event { id, .. })
            | Bpmn::SequenceFlow { id, .. }
            | Bpmn::Activity { id, .. }
            | Bpmn::Definitions { id, .. }
            | Bpmn::Gateway(Gateway { id, .. })
            | Bpmn::Process { id, .. } => id.local_id = value,
            _ => {}
        }
    }

    pub(crate) fn add_output(&mut self, text: String) {
        match self {
            Bpmn::Event(Event { outputs, .. })
            | Bpmn::Gateway(Gateway { outputs, .. })
            | Bpmn::Activity { outputs, .. } => outputs.add(text),
            _ => {}
        }
    }

    pub(crate) fn add_input(&mut self, text: String) {
        if let Bpmn::Gateway(Gateway { inputs, .. }) = self {
            inputs.push(text)
        }
    }
}

impl TryFrom<(&[u8], HashMap<&[u8], String>)> for Bpmn {
    type Error = Error;

    fn try_from(
        (bpmn_type, mut attributes): (&[u8], HashMap<&[u8], String>),
    ) -> Result<Self, Self::Error> {
        let bpmn_type_str = std::str::from_utf8(bpmn_type)?;
        let ty = match bpmn_type {
            DEFINITIONS => Bpmn::Definitions {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                    .into(),
            },
            PROCESS => Bpmn::Process {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                    .into(),
                data_index: None,
                _is_executable: attributes
                    .remove(ATTRIB_IS_EXECUTABLE)
                    .and_then(|s| s.parse::<bool>().ok())
                    .unwrap_or_default(),
            },
            START_EVENT
            | END_EVENT
            | BOUNDARY_EVENT
            | INTERMEDIATE_CATCH_EVENT
            | INTERMEDIATE_THROW_EVENT => Bpmn::Event(Event {
                event_type: bpmn_type.try_into()?,
                symbol: None,
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                    .into(),
                name: attributes.remove(ATTRIB_NAME),
                attached_to_ref: attributes.remove(ATTRIB_ATTACHED_TO_REF).map(Into::into),
                _cancel_activity: attributes.remove(ATTRIB_CANCEL_ACTIVITY),
                outputs: Default::default(),
            }),
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK | SUB_PROCESS | TRANSACTION => {
                Bpmn::Activity {
                    activity_type: bpmn_type.try_into()?,
                    id: attributes
                        .remove(ATTRIB_ID)
                        .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                        .into(),
                    func_idx: None,
                    name: attributes.remove(ATTRIB_NAME),
                    outputs: Default::default(),
                }
            }
            EXCLUSIVE_GATEWAY | PARALLEL_GATEWAY | INCLUSIVE_GATEWAY | EVENT_BASED_GATEWAY => {
                Bpmn::Gateway(Gateway {
                    gateway_type: bpmn_type.try_into()?,
                    id: attributes
                        .remove(ATTRIB_ID)
                        .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                        .into(),
                    func_idx: None,
                    name: attributes.remove(ATTRIB_NAME),
                    default: attributes.remove(ATTRIB_DEFAULT).map(Into::into),
                    outputs: Default::default(),
                    inputs: Default::default(),
                })
            }
            SEQUENCE_FLOW => Bpmn::SequenceFlow {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                    .into(),
                name: attributes.remove(ATTRIB_NAME),
                _source_ref: attributes
                    .remove(ATTRIB_SOURCE_REF)
                    .ok_or(Error::MissingSourceRef)?,
                target_ref: attributes
                    .remove(ATTRIB_TARGET_REF)
                    .ok_or(Error::MissingTargetRef)?
                    .into(),
            },
            INCOMING | OUTGOING => Bpmn::Direction {
                direction_type: bpmn_type.try_into()?,
                text: None,
            },
            _ => return Err(Error::TypeNotImplemented(bpmn_type_str.into())),
        };
        Ok(ty)
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

    pub(crate) fn len(&self) -> usize {
        self.local_ids.len()
    }

    pub(crate) fn first(&self) -> Option<&usize> {
        self.local_ids.first()
    }

    pub(crate) fn update_local_ids(&mut self, bpmn_index: &HashMap<String, usize>) {
        for (idx, value) in self.bpmn_ids.iter().enumerate() {
            if let Some(index) = bpmn_index.get(value) {
                self.local_ids[idx] = *index;
            }
        }
    }

    pub(crate) fn find_by_name_or_id(
        &self,
        value: impl AsRef<str>,
        process_data: &[Bpmn],
    ) -> Option<&usize> {
        self.local_ids.iter().find(|index| {
            if let Some(Bpmn::SequenceFlow { id, name, .. }) = process_data.get(**index) {
                return name.as_deref().is_some_and(|name| name == value.as_ref())
                    || id.bpmn() == value.as_ref();
            }
            false
        })
    }

    pub(crate) fn find_by_intermediate_event(
        &self,
        search: &IntermediateEvent,
        process_data: &[Bpmn],
    ) -> Option<&usize> {
        self.local_ids.iter().find(|index| {
            process_data
                .get(**index)
                .and_then(|bpmn| {
                    if let Bpmn::SequenceFlow { target_ref, .. } = bpmn {
                        return process_data.get(*target_ref.local());
                    }
                    None
                })
                .is_some_and(|bpmn| match bpmn {
                    // We can target both ReceiveTask or Events.
                    Bpmn::Activity {
                        activity_type: ActivityType::ReceiveTask,
                        name: Some(name),
                        ..
                    } => search.1 == Symbol::Message && name.as_str() == search.0,
                    Bpmn::Event(Event {
                        symbol:
                            Some(
                                symbol @ (Symbol::Message
                                | Symbol::Signal
                                | Symbol::Timer
                                | Symbol::Conditional),
                            ),
                        name: Some(name),
                        ..
                    }) => symbol == &search.1 && name.as_str() == search.0,
                    _ => false,
                })
        })
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

    pub(crate) fn update_local_id(&mut self, map: &HashMap<String, usize>) {
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
