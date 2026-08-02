use crate::{
    diagram::{Id, Outputs},
    process::{DiagramErrorKind, RuntimeError},
};
use core::fmt;
use std::{collections::HashMap, fmt::Display};

pub(crate) const DEFINITIONS: &str = "definitions";
pub(crate) const PROCESS: &str = "process";

// Event
pub(crate) const START_EVENT: &str = "startEvent";
pub(crate) const END_EVENT: &str = "endEvent";
pub(crate) const BOUNDARY_EVENT: &str = "boundaryEvent";
pub(crate) const INTERMEDIATE_CATCH_EVENT: &str = "intermediateCatchEvent";
pub(crate) const INTERMEDIATE_THROW_EVENT: &str = "intermediateThrowEvent";

// Event symbol
pub(crate) const CANCEL_EVENT_DEFINITION: &str = "cancelEventDefinition";
pub(crate) const COMPENSATE_EVENT_DEFINITION: &str = "compensateEventDefinition";
pub(crate) const CONDITIONAL_EVENT_DEFINITION: &str = "conditionalEventDefinition";
pub(crate) const ERROR_EVENT_DEFINITION: &str = "errorEventDefinition";
pub(crate) const ESCALATION_EVENT_DEFINITION: &str = "escalationEventDefinition";
pub(crate) const MESSAGE_EVENT_DEFINITION: &str = "messageEventDefinition";
pub(crate) const LINK_EVENT_DEFINITION: &str = "linkEventDefinition";
pub(crate) const SIGNAL_EVENT_DEFINITION: &str = "signalEventDefinition";
pub(crate) const TERMINATE_EVENT_DEFINITION: &str = "terminateEventDefinition";
pub(crate) const TIMER_EVENT_DEFINITION: &str = "timerEventDefinition";

// Task
pub(crate) const TASK: &str = "task";
pub(crate) const SERVICE_TASK: &str = "serviceTask";
pub(crate) const USER_TASK: &str = "userTask";
pub(crate) const SCRIPT_TASK: &str = "scriptTask";
pub(crate) const RECEIVE_TASK: &str = "receiveTask";
pub(crate) const SEND_TASK: &str = "sendTask";
pub(crate) const MANUAL_TASK: &str = "manualTask";
pub(crate) const BUSINESS_RULE_TASK: &str = "businessRuleTask";
pub(crate) const CALL_ACTIVITY: &str = "callActivity";
pub(crate) const SUB_PROCESS: &str = "subProcess";
pub(crate) const TRANSACTION: &str = "transaction";

// Direction
pub(crate) const OUTGOING: &str = "outgoing";
pub(crate) const INCOMING: &str = "incoming";

// Flow
pub(crate) const SEQUENCE_FLOW: &str = "sequenceFlow";

// Gateway
pub(crate) const EXCLUSIVE_GATEWAY: &str = "exclusiveGateway";
pub(crate) const PARALLEL_GATEWAY: &str = "parallelGateway";
pub(crate) const INCLUSIVE_GATEWAY: &str = "inclusiveGateway";
pub(crate) const EVENT_BASED_GATEWAY: &str = "eventBasedGateway";

// Attributes
pub(crate) const ATTRIB_ID: &str = "id";
pub(crate) const _ATTRIB_IS_EXECUTABLE: &str = "isExecutable";
pub(crate) const ATTRIB_NAME: &str = "name";
pub(crate) const _ATTRIB_SOURCE_REF: &str = "sourceRef";
pub(crate) const ATTRIB_TARGET_REF: &str = "targetRef";
pub(crate) const ATTRIB_DEFAULT: &str = "default";
pub(crate) const _ATTRIB_EXPORTER_VERSION: &str = "exporterVersion";
pub(crate) const ATTRIB_ATTACHED_TO_REF: &str = "attachedToRef";
pub(crate) const _ATTRIB_CANCEL_ACTIVITY: &str = "cancelActivity";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum EventType {
    Boundary,
    End,
    IntermediateCatch,
    IntermediateThrow,
    Start,
}

impl TryFrom<&str> for EventType {
    type Error = BpmnError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            BOUNDARY_EVENT => EventType::Boundary,
            END_EVENT => EventType::End,
            INTERMEDIATE_CATCH_EVENT => EventType::IntermediateCatch,
            INTERMEDIATE_THROW_EVENT => EventType::IntermediateThrow,
            START_EVENT => EventType::Start,
            _ => Err(BpmnErrorKind::TypeNotImplemented(value.into()))?,
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

impl TryFrom<&str> for ActivityType {
    type Error = BpmnError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
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
            _ => Err(BpmnErrorKind::TypeNotImplemented(value.into()))?,
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

impl TryFrom<&str> for GatewayType {
    type Error = BpmnError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            EXCLUSIVE_GATEWAY => GatewayType::Exclusive,
            INCLUSIVE_GATEWAY => GatewayType::Inclusive,
            PARALLEL_GATEWAY => GatewayType::Parallel,
            EVENT_BASED_GATEWAY => GatewayType::EventBased,
            _ => Err(BpmnErrorKind::TypeNotImplemented(value.into()))?,
        })
    }
}

impl Display for GatewayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

/// BPMN Symbols
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
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

impl TryFrom<&str> for Symbol {
    type Error = BpmnError;

    fn try_from(value: &str) -> Result<Self, BpmnError> {
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
            _ => Err(BpmnErrorKind::TypeNotImplemented(value.into()))?,
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
    pub(crate) fn default_path(&self) -> Result<&usize, RuntimeError> {
        self.default
            .as_ref()
            .map(Id::local)
            .ok_or_else(|| DiagramErrorKind::MissingDefault(self.to_string()).into())
    }
}

impl Display for Gateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} `{}`",
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
            "{} `{}`",
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
            "{} `{}`",
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

impl TryFrom<(&str, HashMap<&str, String>)> for Bpmn {
    type Error = BpmnError;

    fn try_from(
        (bpmn_type, mut attributes): (&str, HashMap<&str, String>),
    ) -> Result<Self, Self::Error> {
        let ty = match bpmn_type {
            DEFINITIONS => Bpmn::Definitions {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| BpmnErrorKind::MissingId(bpmn_type.into()))?
                    .into(),
            },
            PROCESS => Bpmn::Process {
                id: attributes
                    .remove(ATTRIB_ID)
                    .ok_or_else(|| BpmnErrorKind::MissingId(bpmn_type.into()))?
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
                    .ok_or_else(|| BpmnErrorKind::MissingId(bpmn_type.into()))?
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
                        .ok_or_else(|| BpmnErrorKind::MissingId(bpmn_type.into()))?
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
                        .ok_or_else(|| BpmnErrorKind::MissingId(bpmn_type.into()))?
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
                    .ok_or_else(|| BpmnErrorKind::MissingId(bpmn_type.into()))?
                    .into(),
                name: attributes.remove(ATTRIB_NAME),
                target_ref: attributes
                    .remove(ATTRIB_TARGET_REF)
                    .ok_or(BpmnErrorKind::MissingTargetRef)?
                    .into(),
            },
            INCOMING | OUTGOING => Bpmn::Direction(None),
            _ => Err(BpmnErrorKind::TypeNotImplemented(bpmn_type.into()))?,
        };
        Ok(ty)
    }
}

/// Errors that can occur while constructing bpmn types.
#[derive(thiserror::Error, Debug)]
#[error("could not create bpmn type")]
#[non_exhaustive]
pub struct BpmnError {
    #[from]
    pub source: BpmnErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum BpmnErrorKind {
    #[error("tag `{0}` missing attribute id")]
    MissingId(String),
    #[error("tag `sequenceFlow` missing attribute targetRef")]
    MissingTargetRef,
    #[error("tag `{0}` not implemented")]
    TypeNotImplemented(String),
    #[error(transparent)]
    Encoding(Box<dyn std::error::Error + Send + Sync>),
}
