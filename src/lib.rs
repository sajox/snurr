//! # Snurr
//!
//! `Snurr` can run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.
//! Model a BPMN diagram and use Snurr to run the process flow, just add your own logic in each task and gateway.
//! Snurr contains no database or state over last known position.
//!
//! This is not a complete implementation of the BPMN 2.0 specification but intend to be light weight subset of it.
//!
//! ## Example
//!
//! ### Cargo.toml
//! ```toml
//! [dependencies]
//! snurr = "0.4"
//! log = "0.4"
//! pretty_env_logger = "0.5"
//! ```
//! ### main.rs
//!
//! ```
//! use snurr::{Eventhandler, Process};
//!
//! extern crate pretty_env_logger;
//!
//! #[derive(Debug, Default)]
//! struct Counter {
//!     count: u32,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     pretty_env_logger::init();
//!
//!     // Create process from BPMN file
//!     let bpmn = Process::new("examples/example.bpmn")?;
//!
//!     // Create Eventhandler with struct type
//!     let mut handler: Eventhandler<Counter> = Eventhandler::default();
//!
//!     // Register task function for handler.
//!     handler.add_task("Count 1", |input| {
//!         input.lock().unwrap().count += 1;
//!         Ok(())
//!     });
//!
//!     // Register gateway function for handler.
//!     handler.add_gateway("equal to 3", |input| {
//!         let result = if input.lock().unwrap().count == 3 {
//!             "YES"
//!         } else {
//!             "NO"
//!         };
//!         vec![result]
//!     });
//!
//!     // Run the process with handler and data
//!     let pr = bpmn.run(&handler, Counter::default())?;
//!
//!     // Print the result.
//!     println!("Result: {:?}", pr.result);
//!     Ok(())
//! }
//! ```

mod error;
mod handler;
mod model;
mod process;
mod reader;

pub use handler::{Data, Eventhandler, TaskResult};
pub use model::Symbol;
pub use process::{Process, ProcessResult};
