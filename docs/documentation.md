# Documentation

**Snurr** can run the process flow from a BPMN 2.0 file created by <https://demo.bpmn.io/new>.

- Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
- Change the BPMN diagram with new Task and Gateways without the need to refactor your old code.
- Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
- Contains no database.
- Single or multithreaded (opt in)

This is not a complete implementation of the BPMN 2.0 specification but intend to be a light weight subset of it.

**NOTE** Documentation updated for future version 0.6 release. Documentation might change.

## Migration 

### Version 0.5 -> 0.6 (NOT YET RELEASED)

#### Gateway

Default flow

```rust
vec![] => With::Default
or
vec![] => Default::default
```

One flow

```rust
vec!["YES"] => With::Flow("YES")
or
vec!["YES"] => "YES".into()
```

Many flows

```rust
vec!["YES", "NO"] => With::Fork(vec!["YES", "NO"])
or
vec!["YES", "NO"] => vec!["YES", "NO"].into()
```

#### Task

Updated return type ```Result<(), Symbol>``` to ```Option<Boundary>```

```rust
Symbol::Timer => Boundary(None, Symbol::Timer);
or
Symbol::Timer => Symbol::Timer.into();
```

Match both name and symbol (new)

```rust
Boundary(Some("timeout"), Symbol::Timer);
or
("timeout", Symbol::Timer).into();
```

## Lib

**parallel feature** is disabled by default and might be sufficient. Spawning threads can add additional overhead.

```toml
[dependencies]
snurr = "0.6"
```

With parallel feature enabled, new threads are spawned with parallel, inclusive, task and event forks.

```toml
[dependencies]
snurr = { version = "0.6", features = ["parallel"] }
```

Trace is disabled by default. It start a collector thread for each run.

```toml
[dependencies]
snurr = { version = "0.6", features = ["trace"] }
```


## Process

Create a process by giving a path to a bpmn file. The created process do not mutate and can be run several times. The **ProcessResult** contains the result and a trace. Use scaffold to generate code from the read BPMN file.

### Create and run process

Use our own model to be used by the process.

```rust
#[derive(Debug, Default)]
struct Counter {
    count: u32,
}
```
Read the bpmn file, add the handlers and run the process.

```rust
let bpmn = Process::new("example.bpmn")?;
let mut handler: Eventhandler<Counter> = Eventhandler::default();
// ... Add task and gateway closures here ...
let process_result = bpmn.run(&handler, Counter::default())?;
```

### Replay trace

Run the flow from a previous process run. **NOTE** Requires feature **trace** enabled to actually trace something.

```rust
let trace_result = Process::replay_trace(&handler, Counter::default(), &process_result.trace);
```

### Scaffold

Generate code from all the task and gateways to the given file path with scaffold. Remove scaffold method after file is created.

```rust
let bpmn = Process::new("example.bpmn")?;
bpmn.scaffold("scaffold.rs")?;
```

Output file: **scaffold.rs**

```rust scaffold.rs
// Replace the '()' in the Eventhandler<()> return type with your own type.
pub fn create_handler() -> snurr::Eventhandler<()> {
    let mut handler = snurr::Eventhandler::default();
    handler.add_task("Count 1", |input| None);

    // Exclusive gateway. Names: YES, NO. Flows: Flow_1h0jtl6, Flow_0rsqhpi.
    handler.add_gateway("equal to 3", |input| Default::default());

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

Register task by **name** (if it exist) or **id**. Return a **None** if no boundary is used and follow regular flow.

```rust
handler.add_task("Name or id", |input| {
    None
});
```

If one or more boundaries exist on a task, then a boundary can be returned. If a name exist it must match.

Boundary with no name

```rust
handler.add_task("Name or id", |input| {
    Some(Symbol::Error.into())
});
```
Boundary with name

```rust
handler.add_task("Name or id", |input| {
    Some(("Not good", Symbol::Error).into())
});
```

## Gateways

Only exclusive, event-based and inclusive gateways need to be registered. **Parallel gateways** run **all** available flows. If a gateway name is given then every gateway with same name will use the same closure. Register a gateway by **name** (if it exist) or **id** and return the flow taken by **name** or **id**

![Exclusive, inclusive and parallel gateway](/assets/images/gateways.png)

### Exclusive gateway

![Exclusive gateway](/assets/images/exclusive-gateway.png)

One flow is returned or default.

```rust
handler.add_gateway("CHOOSE", |input| {
    With::Flow("YES")
});
```

```rust
handler.add_gateway("CHOOSE", |input| {
    With::Default
});
```

### Event-based gateway (Version 0.6)

![Event-based gateway](/assets/images/event-based-gateway.png)

One flow is returned.

```rust
handler.add_gateway("CHOOSE", |input| {
     ("Message", Symbol::Message).into()
});
```

### Inclusive gateway

![Inclusive gateway](/assets/images/inclusive-gateway.png)

One or more flows is returned and processed. Inclusive gateway should always have a default flow in the BPMN diagram.

```rust
handler.add_gateway("CHOOSE", |input| {
    With::Fork(vec!["YES", "NO"])
});
```

```rust
handler.add_gateway("CHOOSE", |input| {
    With::Flow("YES")
});
```

```rust
handler.add_gateway("CHOOSE", |input| {
    With::Default
});
```

### Parallel gateway

![Parallel gateway](/assets/images/parallel-gateway.png)

**Parallel gateways** run **all** available flows. No need to add gateway.

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
