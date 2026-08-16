use pretty_env_logger;
use snurr::{ProcessBuilder, Symbol, Task};
use std::sync::{
    Mutex,
    atomic::{AtomicU32, Ordering::Relaxed},
};

// Tasks
const RUN_COUNTER_PROCESS: &'static str = "run counter process";
const COUNT_1: &'static str = "Count 1";

//Gateways
const EQUAL_TO_3: &'static str = "equal to 3";

// Gateway choices
const YES: &'static str = "YES";
const NO: &'static str = "NO";

// Errors
const COUNTER_FAILED: &'static str = "counter failed";

#[derive(Debug, Default)]
struct Counter(AtomicU32);

#[derive(Debug, Default)]
struct Manager {
    counters: Vec<Counter>,
    failed: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let manager = ProcessBuilder::<Mutex<Manager>>::new("examples/reusable.bpmn")?
        .task(RUN_COUNTER_PROCESS, run_counter_process()?)
        .end_event(|input, _name, symbol| {
            // Act on error end event, update model or inform external system
            if let Symbol::Error = symbol {
                input.lock().unwrap().failed = true;
            }
            Ok(())
        })
        .build()?;

    println!("{:?}", manager.run(Default::default())?.into_inner()?);
    Ok(())
}

// Create an external process that is used by another process.
fn run_counter_process() -> Result<impl Fn(&Mutex<Manager>) -> Task, Box<dyn std::error::Error>> {
    // Build up the counter process
    let counter_process = ProcessBuilder::<Counter>::new("examples/counter.bpmn")?
        .task(COUNT_1, |input| {
            input.0.fetch_add(1, Relaxed);
            Default::default()
        })
        .exclusive(EQUAL_TO_3, |input| {
            match input.0.load(Relaxed) {
                3 => YES,
                _ => NO,
            }
            .into()
        })
        .build()?;

    // Move the created counter process into the closure and return the closure.
    Ok(move |input: &Mutex<Manager>| {
        // Run external counter process
        let Ok(counter) = counter_process.run(Default::default()) else {
            return (COUNTER_FAILED, Symbol::Error).into();
        };

        // Store counter result to other process
        input.lock().unwrap().counters.push(counter);
        Default::default()
    })
}
