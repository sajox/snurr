use pretty_env_logger;
use snurr::Process;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

#[derive(Debug, Default)]
struct Counter(AtomicU32);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Create process from BPMN file
    let bpmn = Process::<Counter>::new("examples/example.bpmn")?
        .task("Count 1", |input| {
            input.0.fetch_add(1, Relaxed);
            Default::default()
        })
        .exclusive("equal to 3", |input| {
            match input.0.load(Relaxed) {
                3 => "YES",
                _ => "NO",
            }
            .into()
        })
        .build()?;

    // Run the process with input data
    let result = bpmn.run(Default::default())?;

    // Print the result.
    println!("{result:?}");
    Ok(())
}
