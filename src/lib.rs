//! # Snurr
//!
//! `Snurr` can run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.
//!
//! - Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
//! - Change the BPMN diagram with new Task and Gateways without the need to refactor your old code.
//! - Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
//! - Contains no database.
//! - Single or multithreaded (opt in)
//!
//! This is not a complete implementation of the BPMN 2.0 specification but intend to be a light weight subset of it.
//!
//! ## Example
//!
//! ### Cargo.toml
//! ```toml
//! [dependencies]
//! snurr = "0.6"
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
//!         None
//!     });
//!
//!     // Register gateway function for handler.
//!     handler.add_gateway("equal to 3", |input| {
//!         let result = if input.lock().unwrap().count == 3 {
//!             "YES"
//!         } else {
//!             "NO"
//!         };
//!         result.into()
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

pub use handler::{Data, Eventhandler, TaskResult};
pub use model::{Boundary, Symbol, With};
pub use process::{Process, ProcessResult};
