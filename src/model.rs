use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Arc, Mutex},
};

use log::warn;

use crate::error::Error;

const DEFINITIONS: &[u8] = b"definitions";
const PROCESS: &[u8] = b"process";

// Event
const START_EVENT: &[u8] = b"startEvent";
const END_EVENT: &[u8] = b"endEvent";
const BOUNDARY_EVENT: &[u8] = b"boundaryEvent";
const INTERMEDIATE_CATCH_EVENT: &[u8] = b"intermediateCatchEvent";
const INTERMEDIATE_THROW_EVENT: &[u8] = b"intermediateThrowEvent";

// Event symbol
const ERROR_EVENT_DEFINITION: &[u8] = b"errorEventDefinition";
const MESSAGE_EVENT_DEFINITION: &[u8] = b"messageEventDefinition";
const TIMER_EVENT_DEFINITION: &[u8] = b"timerEventDefinition";
const ESCALATION_EVENT_DEFINITION: &[u8] = b"escalationEventDefinition";
const CONDITIONAL_EVENT_DEFINITION: &[u8] = b"conditionalEventDefinition";
const SIGNAL_EVENT_DEFINITION: &[u8] = b"signalEventDefinition";
const COMPENSATE_EVENT_DEFINITION: &[u8] = b"compensateEventDefinition";
const LINK_EVENT_DEFINITION: &[u8] = b"linkEventDefinition";

// Task
const TASK: &[u8] = b"task";
const SERVICE_TASK: &[u8] = b"serviceTask";
const USER_TASK: &[u8] = b"userTask";
const SCRIPT_TASK: &[u8] = b"scriptTask";
const RECEIVE_TASK: &[u8] = b"receiveTask";
const SEND_TASK: &[u8] = b"sendTask";
const MANUAL_TASK: &[u8] = b"manualTask";
const BUSINESS_RULE_TASK: &[u8] = b"businessRuleTask";
const CALL_ACTIVITY: &[u8] = b"callActivity";
const SUB_PROCESS: &[u8] = b"subProcess";
const TRANSACTION: &[u8] = b"transaction";

// Direction
const OUTGOING: &[u8] = b"outgoing";
const INCOMING: &[u8] = b"incoming";

// Flow
const SEQUENCE_FLOW: &[u8] = b"sequenceFlow";

// Gateway
const EXCLUSIVE_GATEWAY: &[u8] = b"exclusiveGateway";
const PARALLEL_GATEWAY: &[u8] = b"parallelGateway";
const INCLUSIVE_GATEWAY: &[u8] = b"inclusiveGateway";

// Attributes
const ATTRIB_ID: &[u8] = b"id";
const ATTRIB_IS_EXECUTABLE: &[u8] = b"isExecutable";
const ATTRIB_NAME: &[u8] = b"name";
const ATTRIB_SOURCE_REF: &[u8] = b"sourceRef";
const ATTRIB_TARGET_REF: &[u8] = b"targetRef";
const ATTRIB_DEFAULT: &[u8] = b"default";
const ATTRIB_EXPORTER_VERSION: &[u8] = b"exporterVersion";
const ATTRIB_ATTACHED_TO_REF: &[u8] = b"attachedToRef";
const ATTRIB_CANCEL_ACTIVITY: &[u8] = b"cancelActivity";

// Messages
const MISSING_FUNCTION: &str = "Missing function. Please register";

/// Generic type for the task and gateway inputs.
pub type Data<T> = Arc<Mutex<T>>;

/// Task result type
pub type TaskResult = Result<(), Symbol>;

/// Task callback that use `Data` type as input and return a result containing a success unit type ()
/// or a failure that contains a `Symbol` for an alternate flow.
type TaskCallback<T> = Box<dyn Fn(Data<T>) -> TaskResult + Sync>;

/// Gateway callback that use `Data` type as input and return a `Vec` with flow(s) to take
type GatewayCallback<T> = Box<dyn Fn(Data<T>) -> Vec<&'static str> + Sync>;

/// Event handler to add task or gateway functions by name or id
pub struct Eventhandler<T> {
    task_func: HashMap<String, TaskCallback<T>>,
    gateway_func: HashMap<String, GatewayCallback<T>>,
}

impl<T> Default for Eventhandler<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Eventhandler<T> {
    /// Create new empty event handler
    pub fn new() -> Self {
        Self {
            task_func: Default::default(),
            gateway_func: Default::default(),
        }
    }

    /// Add a task to the event handler by name or id with corresponding `TaskCallback`
    pub fn add_task<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        self.task_func.insert(name.into(), Box::new(func));
    }

    pub(crate) fn run_task(&self, key: &str, data: Data<T>) -> TaskResult {
        if let Some(func) = self.task_func.get(key) {
            return (*func)(data);
        } else {
            warn!("{}: {}", MISSING_FUNCTION, key);
        }
        Ok(())
    }

    /// Add a gateway to the event handler by name or id with corresponding `GatewayCallback`
    pub fn add_gateway<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Data<T>) -> Vec<&'static str> + 'static + Sync,
    {
        self.gateway_func.insert(name.into(), Box::new(func));
    }

    pub(crate) fn run_gateway(&self, key: &str, data: Data<T>) -> Vec<&'static str> {
        if let Some(func) = self.gateway_func.get(key) {
            return (*func)(data);
        }
        warn!("{}: {}", MISSING_FUNCTION, key);
        vec![]
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BpmnType {
    StartEvent,
    EndEvent,
    BoundaryEvent,
    IntermediateCatchEvent,
    IntermediateThrowEvent,
    Task,
    Outgoing,
    Incoming,
    ExclusiveGateway,
    Process,
    Definitions,
    SequenceFlow,
    SubProcess,
    ParallelGateway,
    InclusiveGateway,
    ErrorEventDefinition,
    MessageEventDefinition,
    TimerEventDefinition,
    Ignore,
    EscalationEventDefinition,
    ConditionalEventDefinition,
    SignalEventDefinition,
    CompensateEventDefinition,
    LinkEventDefinition,
}

impl Display for BpmnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&[u8]> for BpmnType {
    fn from(value: &[u8]) -> Self {
        match value {
            START_EVENT => BpmnType::StartEvent,
            END_EVENT => BpmnType::EndEvent,
            BOUNDARY_EVENT => BpmnType::BoundaryEvent,
            INTERMEDIATE_CATCH_EVENT => BpmnType::IntermediateCatchEvent,
            INTERMEDIATE_THROW_EVENT => BpmnType::IntermediateThrowEvent,
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK => BpmnType::Task,
            OUTGOING => BpmnType::Outgoing,
            INCOMING => BpmnType::Incoming,
            EXCLUSIVE_GATEWAY => BpmnType::ExclusiveGateway,
            PARALLEL_GATEWAY => BpmnType::ParallelGateway,
            INCLUSIVE_GATEWAY => BpmnType::InclusiveGateway,
            SEQUENCE_FLOW => BpmnType::SequenceFlow,
            PROCESS => BpmnType::Process,
            DEFINITIONS => BpmnType::Definitions,
            SUB_PROCESS | TRANSACTION => BpmnType::SubProcess,
            ERROR_EVENT_DEFINITION => BpmnType::ErrorEventDefinition,
            MESSAGE_EVENT_DEFINITION => BpmnType::MessageEventDefinition,
            TIMER_EVENT_DEFINITION => BpmnType::TimerEventDefinition,
            ESCALATION_EVENT_DEFINITION => BpmnType::EscalationEventDefinition,
            CONDITIONAL_EVENT_DEFINITION => BpmnType::ConditionalEventDefinition,
            SIGNAL_EVENT_DEFINITION => BpmnType::SignalEventDefinition,
            COMPENSATE_EVENT_DEFINITION => BpmnType::CompensateEventDefinition,
            LINK_EVENT_DEFINITION => BpmnType::LinkEventDefinition,
            _ => BpmnType::Ignore,
        }
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

impl TryFrom<(BpmnType, HashMap<BpmnAttrib, String>)> for Bpmn {
    type Error = Error;

    fn try_from(
        (bpmn_type, mut attributes): (BpmnType, HashMap<BpmnAttrib, String>),
    ) -> Result<Self, Self::Error> {
        let ty = match bpmn_type {
            BpmnType::Definitions => Bpmn::Definitions {
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type.to_string()))?,
            },
            BpmnType::Process => Bpmn::Process {
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type.to_string()))?,
                is_executable: attributes
                    .remove(&BpmnAttrib::IsExecutable)
                    .and_then(|s| s.parse::<bool>().ok())
                    .unwrap_or_default(),
                start_id: None,
            },
            BpmnType::StartEvent
            | BpmnType::EndEvent
            | BpmnType::BoundaryEvent
            | BpmnType::IntermediateCatchEvent
            | BpmnType::IntermediateThrowEvent => Bpmn::Event {
                event: bpmn_type,
                symbol: None,
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type.to_string()))?,
                name: attributes.remove(&BpmnAttrib::Name),
                attached_to_ref: attributes.remove(&BpmnAttrib::AttachedToRef),
                cancel_activity: attributes.remove(&BpmnAttrib::CancelActivity),
                output: None,
            },
            BpmnType::Task | BpmnType::SubProcess => Bpmn::Activity {
                aktivity: bpmn_type,
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type.to_string()))?,
                name: attributes.remove(&BpmnAttrib::Name),
                output: None,
                start_id: None,
            },
            BpmnType::ExclusiveGateway | BpmnType::ParallelGateway | BpmnType::InclusiveGateway => {
                Bpmn::Gateway {
                    gateway: bpmn_type,
                    id: attributes
                        .remove(&BpmnAttrib::Id)
                        .ok_or_else(|| Error::MissingId(bpmn_type.to_string()))?,
                    name: attributes.remove(&BpmnAttrib::Name),
                    default: attributes.remove(&BpmnAttrib::Default),
                    outputs: Default::default(),
                }
            }
            BpmnType::SequenceFlow => Bpmn::SequenceFlow {
                id: attributes
                    .remove(&BpmnAttrib::Id)
                    .ok_or_else(|| Error::MissingId(bpmn_type.to_string()))?,
                name: attributes.remove(&BpmnAttrib::Name),
                source_ref: attributes
                    .remove(&BpmnAttrib::SourceRef)
                    .ok_or(Error::MissingSourceRef)?,
                target_ref: attributes
                    .remove(&BpmnAttrib::TargetRef)
                    .ok_or(Error::MissingTargetRef)?,
            },
            BpmnType::Incoming | BpmnType::Outgoing => Bpmn::Direction {
                direction: bpmn_type,
                text: None,
            },
            other => return Err(Error::MissingBpmnType(other.to_string())),
        };
        Ok(ty)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Bpmn {
    Definitions {
        id: String,
    },
    Process {
        id: String,
        is_executable: bool,
        start_id: Option<String>,
    },
    Event {
        event: BpmnType,
        symbol: Option<Symbol>,
        id: String,
        name: Option<String>,
        attached_to_ref: Option<String>,
        cancel_activity: Option<String>,
        output: Option<String>,
    },
    Activity {
        aktivity: BpmnType,
        id: String,
        name: Option<String>,
        output: Option<String>,

        // Only used by subprocess type
        start_id: Option<String>,
    },
    Gateway {
        gateway: BpmnType,
        id: String,
        name: Option<String>,
        default: Option<String>,
        outputs: Vec<String>,
    },
    SequenceFlow {
        id: String,
        name: Option<String>,
        source_ref: String,
        target_ref: String,
    },
    Direction {
        direction: BpmnType,
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

    pub(crate) fn name(&self) -> Option<&String> {
        match self {
            Bpmn::Event { name, .. }
            | Bpmn::Activity { name, .. }
            | Bpmn::Gateway { name, .. }
            | Bpmn::SequenceFlow { name, .. } => name.as_ref(),
            _ => None,
        }
    }

    pub(crate) fn set_output(&mut self, text: String) {
        match self {
            Bpmn::Event { output, .. } | Bpmn::Activity { output, .. } => *output = Some(text),
            Bpmn::Gateway { outputs, .. } => outputs.push(text),
            _ => {}
        }
    }
}

impl From<&Bpmn> for BpmnType {
    fn from(value: &Bpmn) -> Self {
        match value {
            Bpmn::Definitions { .. } => BpmnType::Definitions,
            Bpmn::Process { .. } => BpmnType::Process,
            Bpmn::Event { event, .. } => *event,
            Bpmn::Activity { aktivity, .. } => *aktivity,
            Bpmn::Gateway { gateway, .. } => *gateway,
            Bpmn::SequenceFlow { .. } => BpmnType::SequenceFlow,
            Bpmn::Direction { direction, text: _ } => *direction,
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

impl TryFrom<BpmnType> for Symbol {
    type Error = Error;

    fn try_from(value: BpmnType) -> Result<Self, <Symbol as TryFrom<BpmnType>>::Error> {
        let ty = match value {
            BpmnType::MessageEventDefinition => Symbol::Message,
            BpmnType::TimerEventDefinition => Symbol::Timer,
            BpmnType::EscalationEventDefinition => Symbol::Escalation,
            BpmnType::ConditionalEventDefinition => Symbol::Conditional,
            BpmnType::ErrorEventDefinition => Symbol::Error,
            BpmnType::CompensateEventDefinition => Symbol::Compensation,
            BpmnType::SignalEventDefinition => Symbol::Signal,
            BpmnType::LinkEventDefinition => Symbol::Link,
            _ => return Err(Error::BadSymbolType),
        };
        Ok(ty)
    }
}
