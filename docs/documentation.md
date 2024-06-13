# Documentation

**Snurr** can run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.
Model a BPMN diagram and use Snurr to run the process flow, just add your own logic in each task and gateway.
Snurr contains no database or state over last known position.

This is not a complete implementation of the BPMN 2.0 specification but intend to be light weight subset of it.

## Lib

**parallel feature** is disabled by default and might be sufficient. Spawning threads can add additional overhead.

```toml
[dependencies]
snurr = { git = "https://github.com/sajox/snurr.git"}
```

With parallel feature enabled, new threads are spawned with parallel gateway, task and event "forks".

```toml
[dependencies]
snurr = { git = "https://github.com/sajox/snurr.git", features = ["parallel"] }
```


## Process

Create a process by giving a path to a bpmn file. The created process do not mutate and can be run several times. The **ProcessResult** contains the result and a trace. Use scaffold to generate code from the read BPMN file.

### Usage

#### Create and run process

```rust
#[derive(Debug, Default)]
struct Counter {
    count: u32,
}
```

```rust
let bpmn = Process::new("example.bpmn")?;

// Inspect the created process
dbg!(&bpmn);

let mut handler: Eventhandler<Counter> = Eventhandler::default();

// Run the process
let process_result = bpmn.run(&handler, Counter::default())?;
```

#### Run the flow from a previous process run with Process::replay_trace

```rust
let trace_result = Process::replay_trace(&handler, Counter::default(), &process_result.trace);
```

#### Generate code from all the task and gateways to the given file path with scaffold. Remove scaffold method after file is created.

```rust
let bpmn = Process::new("example.bpmn")?;
bpmn.scaffold("scaffold.rs")?;
```

Output file: **scaffold.rs**

```rust scaffold.rs
pub fn create_handler<T>() -> snurr::Eventhandler<T> {
    let mut handler: snurr::Eventhandler<T> = snurr::Eventhandler::default();
    handler.add_task("Count 1", |input| Ok(()));

    // "equal to 3" output id(s) ["Flow_1h0jtl6", "Flow_0rsqhpi"]
    handler.add_gateway("equal to 3", |input| vec!["NO", "YES"]);

    handler
}
```

## Eventhandler

Create an event handler for a type you want to use as input to task and gateways. Register your own task and gateway closures to the eventhandler and pass it to the process.

### Usage

```rust
#[derive(Debug, Default)]
struct Counter {
    count: u32,
}

let mut handler: Eventhandler<Counter> = Eventhandler::default();
```

## Tasks

All tasks is used in the same way regardless of which icon is used in the BPMN diagram. The input to a task is thread safe. In parallel flows you might need to consider when using and releasing the lock to the input. If a task name is given then every task with same name will use the same closure.

![Tasks](/assets/images/tasks.png)

### Usage

Register task by **name** (if it exist) or **id**. Return a result with a unit tuple if no boundary is used and follow regular flow.

```rust
handler.add_task("Name or id", |input| {
    Ok(())
});
```

If one or more boundarys exist on a task, then a boundary can be returned. Currently only **one** boundary of **same** type can be returned. Might be updated in the future if need arise.

```rust
handler.add_task("Name or id", |input| {
    Err(Symbol::Error)
});
```

## Gateways

Only exclusive and inclusive gateways need to be registered. **Parallel gateways** run **all** available flows. If a gateway name is given then every gateway with same name will use the same closure.


**Exclusive**, **inclusive** and **parallel** gateway

![Exclusive, inclusive and parallel gateway](/assets/images/gateways.png)

### Usage

Register gateway by **name** (if it exist) or **id** and return the flow taken by **name** or **id**

#### Exclusive gateway

![Exclusive gateway](/assets/images/exclusive-gateway.png)

Zero or one flow is returned. If many flows is returned only the first one is processed.

```rust
handler.add_gateway("CHOOSE", |input| {
    vec!["YES"]
});
```

#### Inclusive gateway

![Inclusive gateway](/assets/images/inclusive-gateway.png)

Zero or more flows is returned and processed. Inclusive gateway should always have a default flow in the BPMN diagram.

```rust
handler.add_gateway("CHOOSE", |input| {
    vec!["YES", "NO"]
});
```

#### Parallel gateway

![Parallel gateway](/assets/images/parallel-gateway.png)

**Parallel gateways** run **all** available flows. No need to add gateway.

#### Default flow

To choose the default flow, then return an empty vector. 

**Note** if no default flow is defined then the first available is used. That to be able to test something without implementation.

```rust
handler.add_gateway("Name or id", |input| {
    vec![]
});
```

## Intermediate event

- Intermediate **none** events (no icon) don't do anything and just follow its output. 
- **Link** (throw and catch need a matching name)
- **Other symbols** don't do anything and just follow its output.

Example with message Link throw and catch event:

![Intermediate throw and catch event](/assets/images/intermediate_event.png)
 
## Boundary event

Only interrupting boundary events is implemented and can be used on a task or a sub-process.

Boundary symbols recognized:
- Compensation
- Conditional
- Error
- Escalation
- Link
- Message
- Signal
- Timer

Example with a task error boundary:

![Boundary events](/assets/images/error-boundary.png)

### Usage

If one or more boundary's exist on a task, then a boundary can be returned.


```rust
handler.add_task("Name or id", |input| {
    Err(Symbol::Error)
});
```

## Subprocess

Collapsed, expanded sub-process or transaction can be used.

![Sub-process](/assets/images/subprocess.png)

An end event symbol kan be used in a sub-process to use the boundary as an alternate flow.

![End events](/assets/images/subprocess-message-end.png)

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

## Not supported

### Conditional Sequence Flows

![Conditional Sequence Flows](/tests/not_supported/conditional_sequence_flows.png)

### Both join and fork (Inclusive and Parallel gateway)

![Join and fork](/tests/not_supported/join_and_fork.png)

### Unbalanced Parallel or Inclusive gateway

![Unbalanced Parallel or Inclusive gateway](/tests/not_supported/parallel_unbalanced.png)
