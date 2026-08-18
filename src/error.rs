use std::fmt::{self, Display, Formatter};

pub use crate::diagram::reader::{BpmnError};
pub use crate::process::{
    BpmnFileError, BpmnFileErrorKind, BuildError, DiagramError, ParseError, ParseErrorKind,
    RuntimeError,
};

pub type Result<T> = std::result::Result<T, Error>;

/// Snurr errors
#[derive(Debug)]
pub enum Error {
    BpmnFile(BpmnFileError),
    Parse(ParseError),
    Runtime(RuntimeError),
    Build(BuildError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::BpmnFile(error) => error.fmt(f),
            Error::Parse(error) => error.fmt(f),
            Error::Runtime(error) => error.fmt(f),
            Error::Build(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::BpmnFile(error) => error.source(),
            Error::Parse(error) => error.source(),
            Error::Runtime(error) => error.source(),
            Error::Build(error) => error.source(),
        }
    }
}

impl From<BpmnFileError> for Error {
    fn from(value: BpmnFileError) -> Self {
        Self::BpmnFile(value)
    }
}

impl From<ParseError> for Error {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

impl From<RuntimeError> for Error {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

impl From<BuildError> for Error {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}
