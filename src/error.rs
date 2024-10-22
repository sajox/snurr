pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("BPMN type {0} missing id")]
    MissingId(String),

    #[error("{0} with name or id '{1}' has no output. (Used correct name or id?)")]
    MissingOutput(String, String),

    #[error("{0} with name or id '{1}' has no default flow")]
    MissingDefault(String, String),

    #[error("could not find BPMN data with id {0}")]
    MisssingBpmnData(String),

    #[error("(sub)process missing start id {0}")]
    MissingProcessStart(String),

    #[error("could not find process data with id {0}")]
    MissingProcessData(String),

    #[error("missing definitions id")]
    MissingDefinitionsId,

    #[error("sequenceFlow missing targetRef")]
    MissingTargetRef,

    #[error("sequenceFlow missing sourceRef")]
    MissingSourceRef,

    #[error("type {0} not implemented")]
    TypeNotImplemented(String),

    #[error("could not find {0} boundary symbol attached to {1}")]
    MissingBoundary(String, String),

    #[error("missing intermediate throw event name on {0}")]
    MissingIntermediateThrowEventName(String),

    #[error("missing intermediate catch event symbol {0} with name {1}")]
    MissingIntermediateCatchEvent(String, String),

    #[error("couldn't extract process result")]
    NoProcessResult,

    #[error("{0} not supported")]
    NotSupported(String),

    #[error("{0}")]
    BpmnRequirement(String),

    #[error(transparent)]
    File(#[from] quick_xml::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    Send(#[from] std::sync::mpsc::SendError<(&'static str, String)>),
}
