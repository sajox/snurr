#![doc = include_str!("../docs/documentation.md")]

mod api;
mod bpmn;
mod diagram;
pub mod error;
mod process;

pub use api::{Exclusive, Inclusive, IntermediateEvent, Task};
pub use bpmn::Symbol;
pub use process::{Process, ProcessBuilder};
