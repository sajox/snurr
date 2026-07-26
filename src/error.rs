use crate::process::{BpmnFileError, BuildError, ParseError, RuntimeError};

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
