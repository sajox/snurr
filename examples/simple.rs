use snurr::Process;

extern crate pretty_env_logger;

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    // Create process from BPMN file
    let bpmn = Process::<_, Counter>::new("examples/example.bpmn")?
        .task("Count 1", |input| {
            input.lock().unwrap().count += 1;
            None
        })
        .exclusive("equal to 3", |input| {
            if input.lock().unwrap().count == 3 {
                "YES"
            } else {
                "NO"
            }
            .into()
        })
        .build()?;

    // Run the process with input data
    let counter = bpmn.run(Counter::default())?;

    // Print the result.
    println!("Count: {}", counter.count);
    Ok(())
}
