//! # Snurr
//!
//! `Snurr` can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new>
//! or the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.
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
//! snurr = { git = "https://github.com/sajox/snurr.git" }
//! log = "0.4"
//! pretty_env_logger = "0.5"
//! ```
//! ### main.rs
//!
//! ```
//! use snurr::Process;
//! use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
//! extern crate pretty_env_logger;
//!
//! #[derive(Debug, Default)]
//! struct Counter {
//!     count: AtomicU32,
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     pretty_env_logger::init();
//!
//!     // Create process from BPMN file
//!     let bpmn = Process::<Counter>::new("examples/example.bpmn")?
//!         .task("Count 1", |input| {
//!             input.count.fetch_add(1, Relaxed);
//!             Default::default()
//!         })
//!         .exclusive("equal to 3", |input| {
//!             match input.count.load(Relaxed) {
//!                 3 => "YES",
//!                 _ => "NO",
//!             }
//!             .into()
//!         })
//!         .build()?;
//!
//!     // Run the process with input data
//!     let result = bpmn.run(Default::default())?;
//!
//!     // Print the result.
//!     println!("Count: {}", result.count.load(Relaxed));
//!     Ok(())
//! }
//! ```

mod api;
mod bpmn;
mod diagram;
mod error;
mod process;

pub use api::{Exclusive, Inclusive, IntermediateEvent, Task};
pub use bpmn::Symbol;
pub use error::{Error, Result};
pub use process::{Build, Process, Run};
