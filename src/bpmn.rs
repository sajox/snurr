use crate::{
    diagram::{Id, Outputs},
    error::Error,
};
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
pub(crate) const _ATTRIB_IS_EXECUTABLE: &[u8] = b"isExecutable";
pub(crate) const ATTRIB_NAME: &[u8] = b"name";
pub(crate) const _ATTRIB_SOURCE_REF: &[u8] = b"sourceRef";
pub(crate) const ATTRIB_TARGET_REF: &[u8] = b"targetRef";
pub(crate) const ATTRIB_DEFAULT: &[u8] = b"default";
pub(crate) const _ATTRIB_EXPORTER_VERSION: &[u8] = b"exporterVersion";
pub(crate) const ATTRIB_ATTACHED_TO_REF: &[u8] = b"attachedToRef";
pub(crate) const _ATTRIB_CANCEL_ACTIVITY: &[u8] = b"cancelActivity";

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
    pub(crate) inputs: u16,
}

impl Gateway {
    pub(crate) fn default_path(&self) -> Result<&usize, Error> {
        self.default
            .as_ref()
            .map(Id::local)
            .ok_or_else(|| Error::MissingDefault(self.to_string()))
    }
}

impl Display for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{} "{}""#,
            self.gateway_type,
            self.name.as_deref().unwrap_or(self.id.bpmn())
        )
    }
}

#[derive(Debug)]
pub(crate) struct Event {
    pub(crate) event_type: EventType,
    pub(crate) symbol: Option<Symbol>,
    pub(crate) id: Id,
    pub(crate) name: Option<String>,
    pub(crate) attached_to_ref: Option<Id>,
    pub(crate) outputs: Outputs,
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{} "{}""#,
            self.event_type,
            self.name.as_deref().unwrap_or(self.id.bpmn())
        )
    }
}

#[derive(Debug)]
pub(crate) struct Activity {
    pub(crate) activity_type: ActivityType,
    pub(crate) id: Id,
    pub(crate) func_idx: Option<usize>,
    pub(crate) name: Option<String>,
    pub(crate) outputs: Outputs,
}

impl Display for Activity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{} "{}""#,
            self.activity_type,
            self.name.as_deref().unwrap_or(self.id.bpmn())
        )
    }
}

#[derive(Debug)]
pub(crate) enum Bpmn {
    Activity(Activity),
    Definitions {
        id: Id,
    },
    Direction(Option<String>),
    Event(Event),
    Gateway(Gateway),
    Process {
        id: Id,
        data_index: Option<usize>,
    },
    SequenceFlow {
        id: Id,
        name: Option<String>,
        target_ref: Id,
    },
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
                outputs: Default::default(),
            }),
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK | SUB_PROCESS | TRANSACTION => {
                Bpmn::Activity(Activity {
                    activity_type: bpmn_type.try_into()?,
                    id: attributes
                        .remove(ATTRIB_ID)
                        .ok_or_else(|| Error::MissingId(bpmn_type_str.into()))?
                        .into(),
                    func_idx: None,
                    name: attributes.remove(ATTRIB_NAME),
                    outputs: Default::default(),
                })
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
                target_ref: attributes
                    .remove(ATTRIB_TARGET_REF)
                    .ok_or(Error::MissingTargetRef)?
                    .into(),
            },
            INCOMING | OUTGOING => Bpmn::Direction(None),
            _ => return Err(Error::TypeNotImplemented(bpmn_type_str.into())),
        };
        Ok(ty)
    }
}
