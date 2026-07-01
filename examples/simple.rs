use snurr::Process;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
extern crate pretty_env_logger;

#[derive(Debug, Default)]
struct Counter {
    count: AtomicU32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Create process from BPMN file
    let bpmn = Process::<Counter>::new("examples/example.bpmn")?
        .task("Count 1", |input| {
            input.count.fetch_add(1, Relaxed);
            Default::default()
        })
        .exclusive("equal to 3", |input| {
            match input.count.load(Relaxed) {
                3 => "YES",
                _ => "NO",
            }
            .into()
        })
        .build()?;

    // Run the process with input data
    let result = bpmn.run(Default::default())?;

    // Print the result.
    println!("Count: {}", result.count.load(Relaxed));
    Ok(())
}
