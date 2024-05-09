# Snurr

Create and run the process flow from a BPMN 2.0 file created by https://demo.bpmn.io/new. This is NOT a complete implementation of the BPMN 2.0 specification. Read the [Snurr documentation](#documentation) and explore the **tests** folder for more examples.

## Example

### Usage

```toml
[dependencies]
snurr = { git = "https://github.com/sajox/snurr.git", tag = "0.1.3" }
log = "0.4"
pretty_env_logger = "0.5"
```


```rust
use snurr::{Eventhandler, Process};

extern crate pretty_env_logger;

#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let bpmn = Process::new("examples/example.bpmn")?;
    let mut handler: Eventhandler<Counter> = Eventhandler::default();

    handler.add_task("Count 1", |input| {
        input.lock().unwrap().count += 1;
        Ok(())
    });

    handler.add_gateway("equal to 3", |input| {
        let result = if input.lock().unwrap().count == 3 {
            "YES"
        } else {
            "NO"
        };
        vec![result]
    });

    let pr = bpmn.run(&handler, Counter::default())?;
    println!("Result: {:?}", pr.result);
    Ok(())
}
```

### Output

If RUST_LOG=info is set when running [example](#usage)

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

### Prepared sample

Run or copy the simple.rs in the examples folder

```
RUST_LOG=info cargo run --example simple
```

## Documentation

### Process

Create a process by giving a path to a bpmn file. The created process do not mutate and can be run several times. The **process_result** contains the result and a trace.

#### Usage

Create and run process
```rust
    let bpmn = Process::new("example.bpmn")?;

    // Inspect the created process
    dbg!(&bpmn);

    let mut handler: ...

    // Run the process
    let process_result = bpmn.run(&handler, Counter::default())?;
```

Run the flow from a previous process run with Process::replay_trace

```rust
    let bpmn = Process::new("example.bpmn")?;
    let mut handler: ...
    let process_result = ...

    let trace_result = Process::replay_trace(&handler, Counter::default(), &process_result.trace);
```

### Eventhandler

Create an event handler for a specific type. The type is used as input to different task or gateways.

#### Usage

```rust
    #[derive(Debug, Default)]
    struct Counter {
        count: u32,
    }

    let mut handler: Eventhandler<Counter> = Eventhandler::default();
```

### Tasks

All tasks is used in the same way but may be used to indicate purpose.

![Tasks](/assets/images/tasks.png)

#### Usage

Register task by **name** (if it exist) or **id**. 

```rust
    handler.add_task("Name or id", |input| {
        Ok(())
    });
```

If one or more boundarys exist on a task, then a boundary can be returned.

```rust
    handler.add_task("Name or id", |input| {
        Err(Symbol::Error)
    });
```

### Gateways

Only exclusive and inclusive gateways need to be registered. **Parallel gateways** run **all** available flows.


**Exclusive**, **inclusive** and **parallel** gateway

![Exclusive, inclusive and parallel gateway](/assets/images/gateways.png)

#### Usage

Register gateway by **name** (if it exist) or **id** and return the flow taken by **name** or **id**

Exclusive gateway, one flow taken. If many is returned only first is used.

```rust
    handler.add_gateway("Name or id", |input| {
        vec!["YES"]
    });
```

Inclusive gateway, many flows taken.

```rust
    handler.add_gateway("Name or id", |input| {
        vec!["YES", "NO", "MAYBE"]
    });
```

To choose the default flow, then return an empty vector. 

**Note** if no default path is defined then the first available is used. That to be able to test something without implementation.

```rust
    handler.add_gateway("Name or id", |input| {
        vec![]
    });
```
 
### Boundary event

Only interrupting boundary events is implemented and can be used on a task or a sub-process.

Boundary symbols:
- Blank
- Message
- Timer
- Escalation
- Conditional
- Link
- Error
- Cancel
- Compensation
- Signal
- Multiple
- ParallelMultiple
- Terminate

Example with a task error boundary:

![Boundary events](/assets/images/error-boundary.png)

#### Usage

If one or more boundary's exist on a task, then a boundary can be returned.


```rust
    handler.add_task("Name or id", |input| {
        Err(Symbol::Error)
    });
```

### Subprocess

Collapsed, expanded sub-process or transaction can be used.

![Sub-process](/assets/images/subprocess.png)

An end event symbol kan be used in a sub-process to use the boundary as an alternate flow.

![End events](/assets/images/end-event.png)

### Logging

#### info

```
RUST_LOG=info cargo run
```

#### warn

Identify missing functions in the flow with warn.

```
RUST_LOG=warn cargo run
```
```
 WARN  snurr::model > Missing function. Please register: Count 1
 WARN  snurr::model > Missing function. Please register: equal to 3
```
