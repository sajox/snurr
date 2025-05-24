//! # Snurr
//!
//! `Snurr` can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new>.
//!
//! - Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
//! - Easy to update the BPMN diagram with new Task and Gateways without the need to refactor your old code.
//! - The BPMN file is the actual design. Forget outdated documentation.
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
//! snurr = "0.10"
//! log = "0.4"
//! pretty_env_logger = "0.5"
//! ```
//! ### main.rs
//!
//! ```
//! use snurr::Process;
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
//!     let bpmn = Process::<_, Counter>::new("examples/example.bpmn")?
//!         .task("Count 1", |input| {
//!             input.lock().unwrap().count += 1;
//!             None
//!         })
//!         .exclusive("equal to 3", |input| {
//!             if input.lock().unwrap().count == 3 {
//!                 "YES"
//!             } else {
//!                 "NO"
//!             }
//!             .into()
//!         })
//!         .build()?;
//!
//!     // Run the process with input data
//!     let counter = bpmn.run(Counter::default())?;
//!
//!     // Print the result.
//!     println!("Count: {}", counter.count);
//!     Ok(())
//! }
//! ```

mod error;
mod model;
mod process;

pub use error::{Error, Result};
pub use model::{Boundary, IntermediateEvent, Symbol, With};
pub use process::{Build, Process, Run, handler::Data, handler::TaskResult};
