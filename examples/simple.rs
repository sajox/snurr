use snurr::{Eventhandler, Process};

extern crate pretty_env_logger;

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Create process from BPMN file
    let bpmn = Process::new("examples/example.bpmn")?;

    // Create Eventhandler with struct type
    let mut handler: Eventhandler<Counter> = Eventhandler::default();

    // Register task function for handler.
    handler.add_task("Count 1", |input| {
        input.lock().unwrap().count += 1;
        Ok(())
    });

    // Register gateway function for handler.
    handler.add_gateway("equal to 3", |input| {
        let result = if input.lock().unwrap().count == 3 {
            "YES"
        } else {
            "NO"
        };
        vec![result]
    });

    // Run the process with handler and data
    let pr = bpmn.run(&handler, Counter::default())?;

    // Print the result.
    println!("Result: {:?}", pr.result);
    Ok(())
}
