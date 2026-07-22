# Documentation

**Snurr** can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new> or the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.

- Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
- Easy to update the BPMN diagram with new Task and Gateways without the need to refactor your old code.
- The BPMN file is the actual design. Forget outdated documentation.
- Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
- Contains no database.
- Single or multithreaded (opt in)

This is not a complete implementation of the BPMN 2.0 specification but intend to be a light weight subset of it.

## Changes

### Main branch (BREAKING CHANGES)

- Updated documentation.
- Updated Inclusive API. Renamed Enum `With` to `Inclusive`.
- Updated Exclusive API. New enum type `Exclusive`.
- Updated Task API. New enum type `Task`. Slightly less verbose when a Task Boundary is used.
- Removed Arc and Mutex usage in Snurr and let the user choose. Callbacks now use `&T` instead of `Arc<Mutex<T>>`.
    - Change your process type to `Process::<Arc<Mutex<YourModel>>>::new` to maintain compatibility with existing code.
    - And to extract result
        ```rust
        let data = Arc::into_inner(process_result) // FAIL if Arc has more than one strong reference
                    .ok_or(YourError::NoProcessResult)? 
                    .into_inner() // FAIL if Mutex is poisoned
                    .map_err(|_| YourError::NoProcessResult)?;
        ```
- Removed `Data<T>` type as it was `Arc<Mutex<T>>`.



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
snurr = "x.xx"
```

With parallel feature enabled, new threads are spawned with parallel, inclusive, task and event forks.

```toml
[dependencies]
snurr = { version = "x.xx", features = ["parallel"] }
```

## Process

Create a process by providing a path to a bpmn file. Add tasks and gateways. When `.build()` is called, the BPMN process validates that the required functions are installed. You cannot run a process before `.build()` is called. If `.build()` returns an error, it contains the required functions that are missing. The created process can be run multiple times. 

Use scaffold to generate code from the read BPMN file as a good starting point. Described below.

### Create and run process

Use your own model in the process. It must be **Send + Sync**, regardless of the "parallel" feature is enabled or not. If your model is not `Sync`, you can wrap it in a `Mutex` by specifying `Process::<Mutex<YourModel>>::new`.

```rust
#[derive(Debug, Default)]
struct Counter(AtomicU32);
```
Read the bpmn file, add the behavior and run the process.

```rust
let bpmn = Process::<Counter>::new("examples/example.bpmn")?
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
    .build()?;

let result = bpmn.run(Default::default())?;
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
        // Names: YES, NO. Ids: Flow_1h0jtl6, Flow_0rsqhpi.
        .exclusive("equal to 3", |input| Default::default())
        .build()
}
```

## Tasks

All tasks is used in the same way regardless of which icon is used in the BPMN diagram. If a task name is given then every task with same name will use the same closure. Register a task by **name** (if it exist) or by **id** if no name was given. 

Two or more outgoing sequence flows from a task create a fork of the flow. It is recommended to use a parallel gateway after the task instead, for the sake of clarity.

### Usage

Return **Default** if no boundary is used and follow regular flow.

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

Only branching/forking exclusive, event-based and inclusive gateways need to be added. If a gateway name is given then every gateway with same name and type will use the same closure. Register a gateway by **name** (if it exist) or by **id** if no name was given, and return the outgoing sequence flow taken by **name** or **id**. No merging/joining gateway need to be added from the BPMN diagram with only one output.

Same gateway can do both join and fork instead of using two separate gateways. The latter is recommended for clarity. (i.e two gateways)

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

**Parallel gateways** run **all** available flows. No need to add gateway code. (And you can't)

## Events

### End event

- **None**
- **Terminate** ends the process. In a subprocess, only the subprocess ends and continues with the parent process.
- **Cancel** ends the process in a transaction and run the cancel boundary.
- **Other symbols** can be used in a subprocess to select a subprocess boundary event.

### Intermediate event

- Intermediate **none** events (no icon) don't do anything and just follow its output. 
- **Link** throw and catch need a matching name
- **Other symbols** don't do anything and just follow its output.
 
### Boundary event

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

## Subprocess

Collapsed, expanded sub-process or transaction can be used.

## Not supported

### Conditional Sequence Flows

Use an explicit gateway instead. Snurr return an `Error::NotSupported` if present.

### Unbalanced Inclusive or Parallel gateway construction

Re-write the process with balanced/symmetric gateway pairs. Snurr return an `Error::NotSupported` if occured while running the process. The check is only active in debug mode.
