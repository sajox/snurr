# Documentation

**Snurr** can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new> or the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.

- Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
- Easy to update the BPMN diagram with new Task and Gateways without the need to refactor your old code.
- The BPMN file is the actual design. Forget outdated documentation.
- Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
- Contains no database.
- Single or multithreaded (opt in)

This is not a complete implementation of the BPMN 2.0 specification but intend to be a light weight subset of it.

## Migration

### Version 0.15-wip

- BREAKING CHANGE: Updated Inclusive API. Renamed Enum `With` to `Inclusive`
- BREAKING CHANGE: Updated Exclusive API. New enum type `Exclusive`
- BREAKING CHANGE: Updated Task API. New enum type `Task`. Slightly less verbose when a Task Boundary is used.

#### Old Task
```rust
.task("Name", |input| {
    None
})

.task("Name", |input| {
    Some(Symbol::Error.into())
})

.task("Name", |input| {
    Some(("Not good", Symbol::Error).into())
})
```

#### New Task
```rust
.task("Name", |input| {
    Default::default()
})

.task("Name", |input| {
    Symbol::Error.into()
})

.task("Name", |input| {
    ("Not good", Symbol::Error).into()
})
```

### Version 0.14

- Make the parallel join less permissive for BPMN design errors and respect the number of tokens required before proceeding. Returns an error if gateway is stalled.
- Added support for cancel event. Used in transactions.
- Early detection if multiple none start events is found in same process.
- Removed unused errors.
- Added documentation and images in crates.io release

## Lib

**parallel feature** is disabled by default and might be sufficient. Spawning threads can add additional overhead.

```toml
[dependencies]
snurr = "0.14"
```

With parallel feature enabled, new threads are spawned with parallel, inclusive, task and event forks.

```toml
[dependencies]
snurr = { version = "0.14", features = ["parallel"] }
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
let bpmn = Process::<Counter>::new("example.bpmn")?
        .task("Count 1", |input| {
            input.lock().unwrap().count += 1;
            Default::default()
        })
        .exclusive("equal to 3", |input| {
            match input.lock().unwrap().count {
                3 => "YES",
                _ => "NO",
            }
            .into()
        })
        .build()?;

let result = bpmn.run(Counter::default())?;
```

### Scaffold

Generate code from all the task and gateways to the given file path with scaffold. Remove scaffold method after file is created.

```rust
let bpmn = Process::<Counter>::new("example.bpmn")?;
bpmn.scaffold("scaffold.rs")?;
```

Output file: **scaffold.rs**

```rust scaffold.rs
use snurr::{Error, Process, Run};

// Replace () with your type
pub fn build(process: Process<()>) -> Result<Process<(), Run>, Error> {
    process
        .task("Count 1", |input| Default::default())
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
    Default::default()
})
```

If one or more boundaries exist on a task, then a boundary can be returned. If a name exist it must match.

Boundary with no name

```rust
.task("Name or id", |input| {
    Symbol::Error.into()
})
```
Boundary with name

```rust
.task("Name or id", |input| {
    ("Not good", Symbol::Error).into()
})
```

## Gateways

Only branching/forking exclusive, event-based and inclusive gateways need to be added. If a gateway name is given then every gateway with same name will use the same closure. Register a gateway by **name** (if it exist) or **id** and return the flow taken by **name** or **id**. 

**NOTE** No merging/joining gateway need to be added from the BPMN diagram.

![Exclusive, inclusive and parallel gateway](/assets/images/gateways.png)

### Exclusive gateway

![Exclusive gateway](/assets/images/exclusive-gateway.png)

One flow

```rust
.exclusive("CHOOSE", |input| {
    "YES".into()
})
```
Default flow

```rust
.exclusive("CHOOSE", |input| {
    Default::default()
})
```

### Event-based gateway

![Event-based gateway](/assets/images/event-based-gateway.png)

One flow

```rust
.event_based("CHOOSE", |input| {
     ("Message", Symbol::Message).into()
})
```

### Inclusive gateway

![Inclusive gateway](/assets/images/inclusive-gateway.png)

One or more flows is returned and processed. Inclusive gateway should always have a default flow in the BPMN diagram.

One flow

```rust
.inclusive("CHOOSE", |input| {
    "YES".into()
})
```
Many flows

```rust
.inclusive("CHOOSE", |input| {
    vec!["YES", "NO"].into()
})
```
Default flow

```rust
.inclusive("CHOOSE", |input| {
    Default::default()
})
```

### Parallel gateway

![Parallel gateway](/assets/images/parallel-gateway.png)

**Parallel gateways** run **all** available flows. No need to add gateway. (And you can't)

## End event

![End events](/assets/images/end-events.png)

- **None**
- **Terminate** ends the process. In a subprocess, only the subprocess ends and continues with the parent process.
- **Cancel** ends the process in a transaction and run the cancel boundary.
- **Other symbols** can be used in a subprocess to select a subprocess boundary event.

## Intermediate event

- Intermediate **none** events (no icon) don't do anything and just follow its output. 
- **Link** (throw and catch need a matching name)
- **Other symbols** don't do anything and just follow its output.

Example with message Link throw and catch event:

![Intermediate throw and catch event](/assets/images/intermediate_event.png)
 
## Boundary event

Only interrupting boundary events is implemented and can be used on a task or a sub-process.

Boundary symbols recognized:
- Cancel
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

An end event symbol can be used in a sub-process to use the boundary as an alternate flow.

![End events](/assets/images/subprocess-message.png)

## Logging

### info

```
RUST_LOG=info cargo run
```

## Not supported

### Conditional Sequence Flows

![Conditional Sequence Flows](/tests/not_supported/conditional_sequence_flows.png)

### Unbalanced Inclusive and Parallel gateways

![Unbalanced Inclusive gateway](/tests/not_supported/inclusive_unbalanced.png)
