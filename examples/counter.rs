use pretty_env_logger;
use snurr::ProcessBuilder;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

#[derive(Debug, Default)]
struct Counter(AtomicU32);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Create process from BPMN file
    let bpmn = ProcessBuilder::<Counter>::new("examples/counter.bpmn")?
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
        .end_event(|_input, name, symbol| {
            log::debug!(
                "act on end event `{symbol}` with name `{}`",
                name.unwrap_or_default()
            );
            Ok(())
        })
        .build()?;

    // Run the process with input data and print result
    println!("{:?}", bpmn.run(Default::default())?);
    Ok(())
}
