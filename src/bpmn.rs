use crate::{
    diagram::{
        Id, Outputs,
        reader::{BpmnError, BpmnErrorKind},
    },
    process::{DiagramErrorKind, RuntimeError},
};
use core::fmt;
use std::fmt::Display;

pub(crate) const _DEFINITIONS: &str = "definitions";
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
pub(crate) const ATTRIB_IS_EXECUTABLE: &str = "isExecutable";
pub(crate) const ATTRIB_NAME: &str = "name";
pub(crate) const ATTRIB_SOURCE_REF: &str = "sourceRef";
pub(crate) const ATTRIB_TARGET_REF: &str = "targetRef";
pub(crate) const ATTRIB_DEFAULT: &str = "default";
pub(crate) const ATTRIB_EXPORTER_VERSION: &str = "exporterVersion";
pub(crate) const ATTRIB_ATTACHED_TO_REF: &str = "attachedToRef";
pub(crate) const ATTRIB_CANCEL_ACTIVITY: &str = "cancelActivity";

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Attrib {
    AttachedToRef,
    CancelActivity,
    Default,
    ExporterVersion,
    Id,
    IsExecutable,
    Name,
    SourceRef,
    TargetRef,
}

impl TryFrom<&str> for Attrib {
    type Error = BpmnError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            ATTRIB_ATTACHED_TO_REF => Attrib::AttachedToRef,
            ATTRIB_CANCEL_ACTIVITY => Attrib::CancelActivity,
            ATTRIB_DEFAULT => Attrib::Default,
            ATTRIB_EXPORTER_VERSION => Attrib::ExporterVersion,
            ATTRIB_ID => Attrib::Id,
            ATTRIB_IS_EXECUTABLE => Attrib::IsExecutable,
            ATTRIB_NAME => Attrib::Name,
            ATTRIB_SOURCE_REF => Attrib::SourceRef,
            ATTRIB_TARGET_REF => Attrib::TargetRef,
            _ => Err(BpmnErrorKind::TypeNotImplemented(value.into()))?,
        })
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
    SubProcess,
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
            SUB_PROCESS | TRANSACTION => ActivityType::SubProcess,
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

impl BpmnValidate for Gateway {
    fn validate(&self) -> Result<(), BpmnError> {
        match self {
            Gateway {
                gateway_type: GatewayType::EventBased,
                outputs,
                ..
            } if outputs.len() < 2 => Err(BpmnErrorKind::BpmnRequirement(format!(
                "{} must have at least two outgoing sequence flows",
                self
            )))?,
            Gateway { outputs, .. } if outputs.len() == 0 => {
                Err(BpmnErrorKind::NoOutput(self.to_string()))?
            }
            _ => Ok(()),
        }
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

impl BpmnValidate for Event {
    fn validate(&self) -> Result<(), BpmnError> {
        match self {
            Event {
                event_type:
                    event_type @ (EventType::Boundary
                    | EventType::Start
                    | EventType::IntermediateCatch
                    | EventType::IntermediateThrow),
                symbol,
                outputs,
                ..
            } if !(*event_type == EventType::IntermediateThrow
                && *symbol == Some(Symbol::Link))
                && outputs.len() == 0 =>
            {
                Err(BpmnErrorKind::NoOutput(self.to_string()))?
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Activity {
    pub(crate) activity_type: ActivityType,
    pub(crate) id: Id,
    pub(crate) func_idx: Option<usize>,
    pub(crate) data_index: Option<usize>,
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

impl BpmnValidate for Activity {
    fn validate(&self) -> Result<(), BpmnError> {
        match self {
            Activity { outputs, .. } if outputs.len() == 0 => {
                Err(BpmnErrorKind::NoOutput(self.to_string()))?
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Bpmn {
    Activity(Activity),
    Event(Event),
    Gateway(Gateway),
    SequenceFlow {
        id: Id,
        name: Option<String>,
        target_ref: Id,
    },
}

pub trait BpmnValidate {
    fn validate(&self) -> Result<(), BpmnError>;
}
