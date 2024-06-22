//! # Snurr
//!
//! `Snurr` can run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.
//! Model a BPMN diagram and use Snurr to run the process flow, just add your own logic in each task and gateway.
//! Snurr contains no database or state over last known position.
//!
//! This is not a complete implementation of the BPMN 2.0 specification but intend to be light weight subset of it.

mod error;
mod handler;
mod model;
mod process;
mod reader;

pub use handler::{Data, Eventhandler, TaskResult};
pub use model::Symbol;
pub use process::{Process, ProcessResult};
