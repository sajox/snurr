#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("missing id")]
    MissingId(String),

    #[error("missing output")]
    MissingOutput(String),

    #[error("missing process start")]
    MissingProcessStart,

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

    #[error("file error")]
    File(#[from] quick_xml::Error),

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
}
