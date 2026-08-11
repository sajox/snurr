pub use crate::diagram::reader::{BpmnError, BpmnErrorKind};
pub use crate::process::{
    BpmnFileError, BpmnFileErrorKind, BuildError, DiagramError, DiagramErrorKind, ParseError,
    ParseErrorKind, RuntimeError, RuntimeErrorKind,
};

pub type Result<T> = std::result::Result<T, Error>;

/// Snurr errors
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum Error {
    BpmnFile(#[from] BpmnFileError),
    Parse(#[from] ParseError),
    Runtime(#[from] RuntimeError),
    Build(#[from] BuildError),
    Io(#[from] std::io::Error),
}
