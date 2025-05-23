# Documentation

**Snurr** can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new>.

- Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
- Easy to update the BPMN diagram with new Task and Gateways without the need to refactor your old code.
- The BPMN file is the actual design. Forget outdated documentation.
- Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
- Contains no database.
- Single or multithreaded (opt in)

This is not a complete implementation of the BPMN 2.0 specification but intend to be a light weight subset of it.

## Migration 

### Version 0.9 -> 0.10

- Changed API from AsRef to Into when installing Task or Gateways

### Version 0.8 -> 0.9

- Added builder pattern on Process to register task and gateways.
- Added validation on `.build()` that requires all Tasks and Gateways to have a registered function.
- Changed return types for Task, Exclusive and Event-based gateway. (If tuple conversion was used then no changes is needed.)
- Removed Eventhandler type.
- Removed trace feature (Use the `log` facility instead)
- Removed hashbrown feature (Was used by default)
- Removed replay trace

#### Exclusive gateway

```rust
Some("YES")
// or with conversion
"YES".into()
```

#### Event-based gateway

```rust
IntermediateEvent("Name", Symbol::Message)
// or with conversion
("Name", Symbol::Message).into()
```

#### Task

```rust
Boundary::Symbol(Symbol::Message)
Boundary::NameSymbol("Name", Symbol::Message)
// or with conversion
Symbol::Message.into()
("Name", Symbol::Message).into()
```

## Lib

**parallel feature** is disabled by default and might be sufficient. Spawning threads can add additional overhead.

```toml
[dependencies]
snurr = "0.10"
```

With parallel feature enabled, new threads are spawned with parallel, inclusive, task and event forks.

```toml
[dependencies]
snurr = { version = "0.10", features = ["parallel"] }
```

## Process

Create a process by providing a path to a bpmn file. Add tasks and gateways. When `.build()` is called, the BPMN process validates that the required functions are installed. You cannot run a process before `.build()` is called. If `.build()` returns an error, it contains the required functions that are missing. The created process can be run multiple times. 

Use scaffold to generate code from the read BPMN file as a good starting point. Described below.

### Create and run process

Use your own model to be used by the process. 

```rust
#[derive(Debug, Default)]
struct Counter {
    count: u32,
}
```
Read the bpmn file, add the behavior and run the process.

```rust
let bpmn = Process::<_, Counter>::new("example.bpmn")?
        .task("Count 1", |input| {
            input.lock().unwrap().count += 1;
            None
        })
        .exclusive("equal to 3", |input| {
            let result = if input.lock().unwrap().count == 3 {
                "YES"
            } else {
                "NO"
            };
            result.into()
        })
        .build()?;

let result = bpmn.run(Counter::default())?;
```

### Scaffold

Generate code from all the task and gateways to the given file path with scaffold. Remove scaffold method after file is created.

```rust
let bpmn = Process::new("example.bpmn")?;
bpmn.scaffold("scaffold.rs")?;
```

Output file: **scaffold.rs**

```rust scaffold.rs
fn add_and_build<T>(
    process: snurr::Process<snurr::Build, T>,
) -> Result<snurr::Process<snurr::Run, T>, snurr::Error> {
    process
        .task("Count 1", |input| None)
        // Exclusive gateway. Names: YES, NO. Flows: Flow_1h0jtl6, Flow_0rsqhpi.
        .exclusive("equal to 3", |input| Default::default())
        .build()
}
```

## Tasks

All tasks is used in the same way regardless of which icon is used in the BPMN diagram. The input to a task is thread safe. In parallel flows you might need to consider when using and releasing the lock to the input. If a task name is given then every task with same name will use the same closure.

![Tasks](/assets/images/tasks.png)

### Usage

Register task by **name** (if it exist) or **id**. Return a **None** if no boundary is used and follow regular flow.

```rust
.task("Name or id", |input| {
    None
})
```

If one or more boundaries exist on a task, then a boundary can be returned. If a name exist it must match.

Boundary with no name

```rust
.task("Name or id", |input| {
    Some(Symbol::Error.into())
})
```
Boundary with name

```rust
.task("Name or id", |input| {
    Some(("Not good", Symbol::Error).into())
})
```

## Gateways

Only branching/forking exclusive, event-based and inclusive gateways need to be added. If a gateway name is given then every gateway with same name will use the same closure. Register a gateway by **name** (if it exist) or **id** and return the flow taken by **name** or **id**. 

**NOTE** No merging/joining gateway need to be added from the BPMN diagram.

![Exclusive, inclusive and parallel gateway](/assets/images/gateways.png)

### Exclusive gateway

![Exclusive gateway](/assets/images/exclusive-gateway.png)

One flow is returned or default.

```rust
.exclusive("CHOOSE", |input| {
    "YES".into()
})
```

```rust
.exclusive("CHOOSE", |input| {
    Default::default()
})
```

### Event-based gateway

![Event-based gateway](/assets/images/event-based-gateway.png)

One flow is returned.

```rust
.event_based("CHOOSE", |input| {
     ("Message", Symbol::Message).into()
})
```

### Inclusive gateway

![Inclusive gateway](/assets/images/inclusive-gateway.png)

One or more flows is returned and processed. Inclusive gateway should always have a default flow in the BPMN diagram.

```rust
.inclusive("CHOOSE", |input| {
    With::Fork(vec!["YES", "NO"])
})
```

```rust
.inclusive("CHOOSE", |input| {
    With::Flow("YES")
})
```

```rust
.inclusive("CHOOSE", |input| {
    With::Default
})
```

### Parallel gateway

![Parallel gateway](/assets/images/parallel-gateway.png)

**Parallel gateways** run **all** available flows. No need to add gateway. (And you can't)

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
.task("Name or id", |input| {
    Some(Symbol::Error.into())
});
```

## Subprocess

Collapsed, expanded sub-process or transaction can be used.

![Sub-process](/assets/images/subprocess.png)

An end event symbol kan be used in a sub-process to use the boundary as an alternate flow.

![End events](/assets/images/subprocess-message.png)

## Logging

### info

```
RUST_LOG=info cargo run
```

## Not supported

### Conditional Sequence Flows

![Conditional Sequence Flows](/tests/not_supported/conditional_sequence_flows.png)

### Unbalanced Inclusive gateway

![Unbalanced Inclusive gateway](/tests/not_supported/inclusive_unbalanced.png)
