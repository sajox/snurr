//! # Snurr
//!
//! `Snurr` can create and run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.
//! This is NOT a complete implementation of the BPMN 2.0 specification.

mod error;
mod model;
mod process;
mod reader;

pub use model::{Data, Eventhandler, GatewayCallback, Symbol, TaskCallback};
pub use process::{Process, ProcessResult};
