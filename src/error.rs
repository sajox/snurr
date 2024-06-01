#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing id")]
    MissingId(String),

    #[error("missing output")]
    MissingOutput(String),

    #[error("missing process start")]
    MissingProcessStart,

    #[error("missing process")]
    MissingProcess,

    #[error("missing definitions")]
    MissingDefinitions,

    #[error("missing sub process")]
    MissingSubProcess,

    #[error("missing targetRef")]
    MissingTargetRef,

    #[error("missing sourceRef")]
    MissingSourceRef,

    #[error("missing bpmn type {0}")]
    MissingBpmnType(String),

    #[error("bad gateway output {0}")]
    BadGatewayOutput(String),

    #[error("no gateway output")]
    NoGatewayOutput,

    #[error("missing boundary")]
    MissingBoundary(String),

    #[error("missing name intermediate throw event")]
    MissingNameIntermediateThrowEvent(String),

    #[error("missing intermediate catch event")]
    MissingIntermediateCatchEvent(String),

    #[error("file error")]
    File(#[from] quick_xml::Error),

    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("bad event type")]
    BadEventType,

    #[error("bad symbol type")]
    BadSymbolType,

    #[error("bad activity type")]
    BadActivityType,

    #[error("bad gateway type")]
    BadGatewayType,

    #[error("bad direction type")]
    BadDirectionType,

    #[error("couldn't extract result")]
    NoResult,

    #[error("malformed utf8")]
    Utf8(#[from] std::str::Utf8Error),
}
