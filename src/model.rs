use std::{collections::HashMap, fmt::Display};

use crate::error::Error;

pub(crate) const DEFINITIONS: &[u8] = b"definitions";
pub(crate) const PROCESS: &[u8] = b"process";

// Event
pub(crate) const START_EVENT: &[u8] = b"startEvent";
pub(crate) const END_EVENT: &[u8] = b"endEvent";
pub(crate) const BOUNDARY_EVENT: &[u8] = b"boundaryEvent";
pub(crate) const INTERMEDIATE_CATCH_EVENT: &[u8] = b"intermediateCatchEvent";
pub(crate) const INTERMEDIATE_THROW_EVENT: &[u8] = b"intermediateThrowEvent";

// Event symbol
pub(crate) const ERROR_EVENT_DEFINITION: &[u8] = b"errorEventDefinition";
pub(crate) const MESSAGE_EVENT_DEFINITION: &[u8] = b"messageEventDefinition";
pub(crate) const TIMER_EVENT_DEFINITION: &[u8] = b"timerEventDefinition";
pub(crate) const ESCALATION_EVENT_DEFINITION: &[u8] = b"escalationEventDefinition";
pub(crate) const CONDITIONAL_EVENT_DEFINITION: &[u8] = b"conditionalEventDefinition";
pub(crate) const SIGNAL_EVENT_DEFINITION: &[u8] = b"signalEventDefinition";
pub(crate) const COMPENSATE_EVENT_DEFINITION: &[u8] = b"compensateEventDefinition";
pub(crate) const LINK_EVENT_DEFINITION: &[u8] = b"linkEventDefinition";

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

// Attributes
pub(crate) const ATTRIB_ID: &[u8] = b"id";
pub(crate) const ATTRIB_IS_EXECUTABLE: &[u8] = b"isExecutable";
pub(crate) const ATTRIB_NAME: &[u8] = b"name";
pub(crate) const ATTRIB_SOURCE_REF: &[u8] = b"sourceRef";
pub(crate) const ATTRIB_TARGET_REF: &[u8] = b"targetRef";
pub(crate) const ATTRIB_DEFAULT: &[u8] = b"default";
pub(crate) const ATTRIB_EXPORTER_VERSION: &[u8] = b"exporterVersion";
pub(crate) const ATTRIB_ATTACHED_TO_REF: &[u8] = b"attachedToRef";
pub(crate) const ATTRIB_CANCEL_ACTIVITY: &[u8] = b"cancelActivity";

#[derive(Debug, Copy, Clone)]
pub(crate) enum EventType {
    Start,
    End,
    Boundary,
    IntermediateCatch,
    IntermediateThrow,
}

impl TryFrom<&[u8]> for EventType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            START_EVENT => EventType::Start,
            END_EVENT => EventType::End,
            BOUNDARY_EVENT => EventType::Boundary,
            INTERMEDIATE_CATCH_EVENT => EventType::IntermediateCatch,
            INTERMEDIATE_THROW_EVENT => EventType::IntermediateThrow,
            _ => return Err(Error::BadEventType),
        })
    }
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum ActivityType {
    Task,
    SubProcess,
}

impl TryFrom<&[u8]> for ActivityType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK => ActivityType::Task,
            SUB_PROCESS | TRANSACTION => ActivityType::SubProcess,
            _ => return Err(Error::BadActivityType),
        })
    }
}

impl Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum GatewayType {
    Exclusive,
    Parallel,
    Inclusive,
}

impl TryFrom<&[u8]> for GatewayType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            EXCLUSIVE_GATEWAY => GatewayType::Exclusive,
            PARALLEL_GATEWAY => GatewayType::Parallel,
            INCLUSIVE_GATEWAY => GatewayType::Inclusive,
            _ => return Err(Error::BadGatewayType),
        })
    }
}

impl Display for GatewayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum DirectionType {
    Outgoing,
    Incoming,
}

impl TryFrom<&[u8]> for DirectionType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            OUTGOING => DirectionType::Outgoing,
            INCOMING => DirectionType::Incoming,
            _ => return Err(Error::BadDirectionType),
        })
    }
}

impl Display for DirectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum BpmnAttrib {
    Id,
    IsExecutable,
    Name,
    SourceRef,
    TargetRef,
    Default,
    ExporterVersion,
    AttachedToRef,
    CancelActivity,
    Ignore,
}

impl From<&[u8]> for BpmnAttrib {
    fn from(value: &[u8]) -> Self {
        match value {
            ATTRIB_ID => BpmnAttrib::Id,
            ATTRIB_IS_EXECUTABLE => BpmnAttrib::IsExecutable,
            ATTRIB_NAME => BpmnAttrib::Name,
            ATTRIB_SOURCE_REF => BpmnAttrib::SourceRef,
            ATTRIB_TARGET_REF => BpmnAttrib::TargetRef,
            ATTRIB_DEFAULT => BpmnAttrib::Default,
            ATTRIB_EXPORTER_VERSION => BpmnAttrib::ExporterVersion,
            ATTRIB_ATTACHED_TO_REF => BpmnAttrib::AttachedToRef,
            ATTRIB_CANCEL_ACTIVITY => BpmnAttrib::CancelActivity,
            _ => BpmnAttrib::Ignore,
        }
    }
}

/// BPMN Symbols
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Symbol {
    Message,
    Timer,
    Escalation,
    Conditional,
    Link,
    Error,
    Cancel,
    Compensation,
    Signal,
    Multiple,
    ParallelMultiple,
    Terminate,
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<&[u8]> for Symbol {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Error> {
        let ty = match value {
            MESSAGE_EVENT_DEFINITION => Symbol::Message,
            TIMER_EVENT_DEFINITION => Symbol::Timer,
            ESCALATION_EVENT_DEFINITION => Symbol::Escalation,
            CONDITIONAL_EVENT_DEFINITION => Symbol::Conditional,
            ERROR_EVENT_DEFINITION => Symbol::Error,
            COMPENSATE_EVENT_DEFINITION => Symbol::Compensation,
            SIGNAL_EVENT_DEFINITION => Symbol::Signal,
            LINK_EVENT_DEFINITION => Symbol::Link,
            _ => return Err(Error::BadSymbolType),
        };
        Ok(ty)
    }
}

#[derive(Debug)]
pub(crate) enum Bpmn {
    Definitions {
        id: String,
    },
    Process {
        id: String,
        _is_executable: bool,
        start_id: Option<String>,
    },
    Event {
        event: EventType,
        symbol: Option<Symbol>,
        id: String,
        name: Option<String>,
        attached_to_ref: Option<String>,
        _cancel_activity: Option<String>,
        outputs: Outputs,
        inputs: Vec<String>,
    },
    Activity {
        aktivity: ActivityType,
        id: String,
        name: Option<String>,
        outputs: Outputs,
        inputs: Vec<String>,

        // Only used by subprocess type
        start_id: Option<String>,
    },
    Gateway {
        gateway: GatewayType,
        id: String,
        name: Option<String>,
        default: Option<String>,
        outputs: Outputs,
        inputs: Vec<String>,
    },
    SequenceFlow {
        id: String,
        name: Option<String>,
        source_ref: String,
        target_ref: String,
    },
    Direction {
        _direction: DirectionType,
        text: Option<String>,
    },
}

impl Bpmn {
    pub(crate) fn id(&self) -> Option<&String> {
        match self {
            Bpmn::Definitions { id, .. }
            | Bpmn::Process { id, .. }
            | Bpmn::Event { id, .. }
            | Bpmn::Activity { id, .. }
            | Bpmn::Gateway { id, .. }
            | Bpmn::SequenceFlow { id, .. } => Some(id),
            Bpmn::Direction { .. } => None,
        }
    }

    pub(crate) fn set_output(&mut self, text: String) {
        match self {
            Bpmn::Event { outputs, .. }
            | Bpmn::Gateway { outputs, .. }
            | Bpmn::Activity { outputs, .. } => outputs.add(text),
            _ => {}
        }
    }

    pub(crate) fn set_input(&mut self, text: String) {
        match self {
            Bpmn::Event { inputs, .. }
            | Bpmn::Gateway { inputs, .. }
            | Bpmn::Activity { inputs, .. } => inputs.push(text),
            _ => {}
        }
    }
}

impl TryFrom<(&[u8], HashMap<BpmnAttrib, String>)> for Bpmn {
    type Error = Error;

    fn try_from(
        (bpmn_type, mut attributes): (&[u8], HashMap<BpmnAttrib, String>),
    ) -> Result<Self, Self::Error> {
        let bpmn_type_str = std::str::from_utf8(bpmn_type)?;
        let ty = match bpmn_type {
            DEFINITIONS => Bpmn::Definitions {
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
            },
            PROCESS => Bpmn::Process {
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                _is_executable: attributes
                    .remove(&BpmnAttrib::IsExecutable)
                    .and_then(|s| s.parse::<bool>().ok())
                    .unwrap_or_default(),
                start_id: None,
            },
            START_EVENT
            | END_EVENT
            | BOUNDARY_EVENT
            | INTERMEDIATE_CATCH_EVENT
            | INTERMEDIATE_THROW_EVENT => Bpmn::Event {
                event: bpmn_type.try_into()?,
                symbol: None,
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                name: attributes.remove(&BpmnAttrib::Name),
                attached_to_ref: attributes.remove(&BpmnAttrib::AttachedToRef),
                _cancel_activity: attributes.remove(&BpmnAttrib::CancelActivity),
                outputs: Default::default(),
                inputs: Default::default(),
            },
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK | SUB_PROCESS | TRANSACTION => {
                Bpmn::Activity {
                    aktivity: bpmn_type.try_into()?,
                    id: attributes
                        .remove(&BpmnAttrib::Id)
                        .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                    name: attributes.remove(&BpmnAttrib::Name),
                    outputs: Default::default(),
                    inputs: Default::default(),
                    start_id: None,
                }
            }
            EXCLUSIVE_GATEWAY | PARALLEL_GATEWAY | INCLUSIVE_GATEWAY => Bpmn::Gateway {
                gateway: bpmn_type.try_into()?,
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                name: attributes.remove(&BpmnAttrib::Name),
                default: attributes.remove(&BpmnAttrib::Default),
                outputs: Default::default(),
                inputs: Default::default(),
            },
            SEQUENCE_FLOW => Bpmn::SequenceFlow {
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                name: attributes.remove(&BpmnAttrib::Name),
                source_ref: attributes
                    .remove(&BpmnAttrib::SourceRef)
                    .ok_or(Error::MissingSourceRef)?,
                target_ref: attributes
                    .remove(&BpmnAttrib::TargetRef)
                    .ok_or(Error::MissingTargetRef)?,
            },
            INCOMING | OUTGOING => Bpmn::Direction {
                _direction: bpmn_type.try_into()?,
                text: None,
            },
            _ => return Err(Error::MissingBpmnType(bpmn_type_str.into())),
        };
        Ok(ty)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Outputs {
    names: Vec<Option<String>>,
    ids: Vec<String>,
}

impl Outputs {
    fn add(&mut self, output_id: impl Into<String>) {
        self.ids.push(output_id.into());
        self.names.push(None);
    }

    pub(crate) fn register_name(&mut self, search_id: impl AsRef<str>, name: impl Into<String>) {
        if let Some((index, _)) = self
            .ids
            .iter()
            .enumerate()
            .find(|(_, id)| id.as_str() == search_id.as_ref())
        {
            if let Some(value) = self.names.get_mut(index) {
                *value = Some(name.into())
            }
        }
    }

    pub(crate) fn name_to_id(&self, search_name: impl AsRef<str>) -> Option<&str> {
        if let Some((index, _)) = self
            .names
            .iter()
            .enumerate()
            .find(|(_, name)| name.as_deref() == Some(search_name.as_ref()))
        {
            return self.ids.get(index).map(|s| s.as_str());
        }
        None
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().filter_map(|s| s.as_deref())
    }

    pub(crate) fn ids(&self) -> impl Iterator<Item = &str> {
        self.ids.iter().map(String::as_str)
    }

    pub(crate) fn len(&self) -> usize {
        self.ids.len()
    }

    pub(crate) fn first(&self) -> Option<&String> {
        self.ids.first()
    }
}
