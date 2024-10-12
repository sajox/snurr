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

#[derive(Debug, Copy, Clone)]
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
                ))
            }
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
    SubProcess,
    Task,
}

impl TryFrom<&[u8]> for ActivityType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            SUB_PROCESS | TRANSACTION => ActivityType::SubProcess,
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK => ActivityType::Task,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ))
            }
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
    Inclusive,
    Parallel,
}

impl TryFrom<&[u8]> for GatewayType {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(match value {
            EXCLUSIVE_GATEWAY => GatewayType::Exclusive,
            INCLUSIVE_GATEWAY => GatewayType::Inclusive,
            PARALLEL_GATEWAY => GatewayType::Parallel,
            _ => {
                return Err(Error::TypeNotImplemented(
                    std::str::from_utf8(value)?.into(),
                ))
            }
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
                ))
            }
        })
    }
}

impl Display for DirectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
        write!(f, "{:?}", self)
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
                ))
            }
        };
        Ok(ty)
    }
}

#[derive(Debug)]
pub(crate) enum Bpmn {
    Activity {
        activity: ActivityType,
        id: String,
        name: Option<String>,
        outputs: Outputs,
    },
    Definitions {
        id: String,
    },
    Direction {
        direction: DirectionType,
        text: Option<String>,
    },
    Event {
        event: EventType,
        symbol: Option<Symbol>,
        id: BpmnLocal,
        name: Option<String>,
        attached_to_ref: Option<BpmnLocal>,
        _cancel_activity: Option<String>,
        outputs: Outputs,
    },
    Gateway {
        gateway: GatewayType,
        id: String,
        name: Option<String>,
        default: Option<BpmnLocal>,
        outputs: Outputs,
        inputs: Vec<String>,
    },
    Process {
        id: String,
        _is_executable: bool,
    },
    SequenceFlow {
        id: String,
        name: Option<String>,
        _source_ref: String,
        target_ref: BpmnLocal,
    },
}

impl Bpmn {
    pub(crate) fn id(&self) -> Result<&str, Error> {
        match self {
            Bpmn::Definitions { id, .. }
            | Bpmn::Process { id, .. }
            | Bpmn::Activity { id, .. }
            | Bpmn::Gateway { id, .. }
            | Bpmn::SequenceFlow { id, .. } => Ok(id),
            Bpmn::Event { id, .. } => Ok(id.bpmn()),
            Bpmn::Direction { direction, .. } => Err(Error::MissingId(direction.to_string())),
        }
    }

    pub(crate) fn add_output(&mut self, text: String) {
        match self {
            Bpmn::Event { outputs, .. }
            | Bpmn::Gateway { outputs, .. }
            | Bpmn::Activity { outputs, .. } => outputs.add(text),
            _ => {}
        }
    }

    // Only use input to check unsupported join and fork
    pub(crate) fn add_input(&mut self, text: String) {
        if let Bpmn::Gateway { inputs, .. } = self {
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
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
            },
            PROCESS => Bpmn::Process {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                _is_executable: attributes
                    .remove(ATTRIB_IS_EXECUTABLE)
                    .and_then(|s| s.parse::<bool>().ok())
                    .unwrap_or_default(),
            },
            START_EVENT
            | END_EVENT
            | BOUNDARY_EVENT
            | INTERMEDIATE_CATCH_EVENT
            | INTERMEDIATE_THROW_EVENT => Bpmn::Event {
                event: bpmn_type.try_into()?,
                symbol: None,
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                    .into(),
                name: attributes.remove(ATTRIB_NAME),
                attached_to_ref: attributes.remove(ATTRIB_ATTACHED_TO_REF).map(Into::into),
                _cancel_activity: attributes.remove(ATTRIB_CANCEL_ACTIVITY),
                outputs: Default::default(),
            },
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK | SUB_PROCESS | TRANSACTION => {
                Bpmn::Activity {
                    activity: bpmn_type.try_into()?,
                    id: attributes
                        .remove(ATTRIB_ID)
                        .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                    name: attributes.remove(ATTRIB_NAME),
                    outputs: Default::default(),
                }
            }
            EXCLUSIVE_GATEWAY | PARALLEL_GATEWAY | INCLUSIVE_GATEWAY => Bpmn::Gateway {
                gateway: bpmn_type.try_into()?,
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
                name: attributes.remove(ATTRIB_NAME),
                default: attributes.remove(ATTRIB_DEFAULT).map(Into::into),
                outputs: Default::default(),
                inputs: Default::default(),
            },
            SEQUENCE_FLOW => Bpmn::SequenceFlow {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?,
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
                direction: bpmn_type.try_into()?,
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

impl Outputs {
    fn add(&mut self, output_id: impl Into<String>) {
        self.bpmn_ids.push(output_id.into());
        self.local_ids.push(0);
    }

    pub(crate) fn ids(&self) -> Vec<&usize> {
        self.local_ids.iter().collect()
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
}

#[derive(Debug)]
pub(crate) struct BpmnLocal(pub(crate) String, pub(crate) usize);

impl Display for BpmnLocal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {})", self.0, self.1)
    }
}

impl BpmnLocal {
    pub(crate) fn bpmn(&self) -> &str {
        &self.0
    }

    pub(crate) fn local(&self) -> &usize {
        &self.1
    }
}

impl From<String> for BpmnLocal {
    fn from(value: String) -> Self {
        Self(value, 0)
    }
}
