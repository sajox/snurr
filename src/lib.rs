//! # Snurr
//!
//! `Snurr` can create and run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.
//! This is not a complete implementation of the BPMN 2.0 specification but a useful subset of it.

mod error;
mod model;
mod process;
mod reader;
mod scaffold;

pub use model::{Data, Eventhandler, Symbol, TaskResult};
pub use process::{Process, ProcessResult};
