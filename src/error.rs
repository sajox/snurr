pub type Result<T> = std::result::Result<T, Error>;

/// Snurr errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Errors that can occur while creating a new process
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("BPMN type {0} missing id")]
    MissingId(String),
    #[error("sequenceFlow missing targetRef")]
    MissingTargetRef,
    #[error("type {0} not implemented")]
    TypeNotImplemented(String),
    #[error("{0} not supported")]
    NotSupported(String),
    #[error("{0}")]
    BpmnRequirement(String),
    #[error("{0}")]
    Builder(String),
    #[error(transparent)]
    BpmnFile(Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

impl From<quick_xml::Error> for ParseError {
    fn from(value: quick_xml::Error) -> Self {
        ParseError::BpmnFile(Box::new(value))
    }
}

/// Errors that can occur while running a process
#[derive(thiserror::Error, Debug)]
pub enum RuntimeError {
    #[error("{0} has no output. (Used correct name or id?)")]
    MissingOutput(String),
    #[error("{0} has no implementation")]
    MissingImplementation(String),
    #[error("{0} has no default flow")]
    MissingDefault(String),
    #[error("could not find BPMN data with id {0}")]
    MisssingBpmnData(String),
    #[error("could not find process data with id {0}")]
    MissingProcessData(String),
    #[error("missing definitions id")]
    MissingDefinitionsId,
    #[error("type {0} not implemented")]
    TypeNotImplemented(String),
    #[error("could not find {0} boundary symbol attached to {1}")]
    MissingBoundary(String, String),
    #[error("{0} could not find {1}")]
    MissingIntermediateEvent(String, String),
    #[error("missing intermediate throw event name on {0}")]
    MissingIntermediateThrowEventName(String),
    #[error("missing intermediate catch event symbol {0} with name {1}")]
    MissingIntermediateCatchEvent(String, String),
    #[error("missing end event")]
    MissingEndEvent,
    #[error("missing start event")]
    MissingStartEvent,
    #[error("{0} not supported")]
    NotSupported(String),
    #[error("{0}")]
    BpmnRequirement(String),
}

/// Errors that can occur while trying to build a process to make it runnable.
#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Missing implementations {0}")]
    MissingImplementations(String),
    #[error("Handlermap has already been consumed")]
    MapConsumed,
}

// BpmnRequirement
pub(crate) const AT_LEAST_TWO_OUTGOING: &str =
    "Event gateway must have at least two outgoing sequence flows";
pub(crate) const ONLY_ONE_START_EVENT: &str = "There can only be one start event of type none";

// Builder
pub(crate) const BUILD_PROCESS_ERROR_MSG: &str = "Couldn't build process";
pub(crate) const XML_ERROR_MSG: &str = "XML error(s) found. Check logs.";
