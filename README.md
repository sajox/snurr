# Snurr

Create and run the process flow from a BPMN 2.0 file created by https://demo.bpmn.io/new. This is NOT a complete implementation of the BPMN 2.0 specification but may still be useful.

Take a look in the **tests** folder for example files.

- Exclusive, inclusive and parallell gateways
- Boundary symbols
- Subprocess


## Example

### Run
```
RUST_LOG=info cargo run --example simple
```

## Usage

- A task or gateway can be registered by **name** (if it exist) or **id**. 
- Only exclusive and inclusive gateways need to be registered. Paralllell gateways run all paths.
- Exclusive gateway should return the selected flow. Only first is used if many.
- Inclusive gateway can return many **name** or **id**.

### Clone this project and reference it in your project cargo.toml
```toml
[dependencies]
snurr = { git = "https://github.com/sajox/Snurr.git", tag = "0.1.0" }
log = "0.4"
pretty_env_logger = "0.5"
```

### Sample code

```rust
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
```

### Output
```
 INFO  snurr::process > StartEvent: Begin process
 INFO  snurr::process > SequenceFlow: count
 INFO  snurr::process > Task: Count 1
 INFO  snurr::process > SequenceFlow: control
 INFO  snurr::process > ExclusiveGateway: equal to 3
 INFO  snurr::process > SequenceFlow: NO
 INFO  snurr::process > Task: Count 1
 INFO  snurr::process > SequenceFlow: control
 INFO  snurr::process > ExclusiveGateway: equal to 3
 INFO  snurr::process > SequenceFlow: NO
 INFO  snurr::process > Task: Count 1
 INFO  snurr::process > SequenceFlow: control
 INFO  snurr::process > ExclusiveGateway: equal to 3
 INFO  snurr::process > SequenceFlow: YES
 INFO  snurr::process > EndEvent: End process
Result: Counter { count: 3 }
```

## Logging

### info
```
RUST_LOG=info cargo run
```

### warn

Identify missing functions in the flow with warn.

```
RUST_LOG=warn cargo run
```
```
 WARN  snurr::model > Missing function. Please register: Count 1
 WARN  snurr::model > Missing function. Please register: equal to 3
```
