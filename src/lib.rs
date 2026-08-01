#![doc = include_str!("../docs/documentation.md")]

mod api;
mod bpmn;
mod diagram;
mod error;
pub mod process;

pub use api::{Exclusive, Inclusive, IntermediateEvent, Task};
pub use bpmn::Symbol;
pub use error::{Error, Result};
pub use process::{Build, Process, Run};
