**Snurr** is a lightweight workflow engine that can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new> or the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.

**How to:**

1. Create your BPMN diagram.
2. Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
3. Add custom behavior using [Rust](https://rust-lang.org) code from a small API. The wiring is already setup from the file.
4. Run your process in single or multi-threaded mode.

**Maintainability:**

- Update the BPMN diagram with new activities and gateways to meet changing requirements. The code is loosely coupled.
- The BPMN file is the actual design. Forget outdated documentation.
- No complicated configuration or database requirements.

This is not intended to be a full-fledged BPMN 2.0 solution, but rather a solution that is easy to embed and use. Read the documentation on what is supported.

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

Create a process builder and initialize it from the BPMN file path. May return an error if the file was not found or if there were problems with the BPMN content. Add the tasks and gateways specified in your BPMN diagram.

When `.build()` is called, the builder validates that the required functions/closures are installed and return a runnable process if successful. If `.build()` returns an error, it contains the required functions that are missing. The created process can be run multiple times.

Use scaffold to generate code from the read BPMN file as a good starting point. Described below.

### Create and run process

Use your own model in the process builder. It must be **Send + Sync**, regardless of the "parallel" feature is enabled or not. If your model is not `Sync`, you can wrap it in a `Mutex` by specifying `ProcessBuilder::<Mutex<YourModel>>::new`.

Read the bpmn file, add the behavior and run the process.

```rust
use snurr::ProcessBuilder;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

#[derive(Debug, Default)]
struct Counter(AtomicU32);

fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Create process from BPMN file
    let bpmn = ProcessBuilder::<Counter>::new("examples/counter.bpmn")?
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

    // Run the process with input data
    let result = bpmn.run(Default::default())?;

    // Print the result.
    println!("{result:?}");
    Ok(())
}
```

### Scaffold

Generate code from all the task and gateways to the given file path with scaffold. Returns an error message if the file already exists. Remove scaffold call after file is created.

```rust no_run
use snurr::ProcessBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bpmn: ProcessBuilder<()> = ProcessBuilder::new("examples/counter.bpmn")?;
    bpmn.scaffold("examples/scaffold.rs")?;
    Ok(())
}
```

Output file: **scaffold.rs**

```rust no_run
use snurr::{Process, ProcessBuilder, error::BuildError};

// Replace () with your type
pub fn build(process_builder: ProcessBuilder<()>) -> Result<Process<()>, BuildError> {
    process_builder
        .task("Count 1", |input| Default::default())
        // outputs: YES, NO
        .exclusive("equal to 3", |input| Default::default())
        .build()
}
```

## Tasks

All tasks is used in the same way regardless of which icon is used in the BPMN diagram. If a task name is given then every task with same name will use the same closure. Register a task by **name** or by **id**. A name is preferable, since an id can be regenerated in the BPMN tool (if elements are deleted and re-added).

Two or more outgoing sequence flows from a task create a fork of the flow. It is recommended to use a parallel gateway after the task instead, for the sake of clarity.

### Task

#### Default flow

Return `Default` if no boundary is used and follow regular flow.

```rust no_run
# use snurr::ProcessBuilder;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.task("name or id", |input| {
    Default::default()
});
# Ok(())
# }
```

#### Boundary flow

If one or more boundaries exist on a task, then a boundary can be returned. If a name exist it must match.

##### Boundary with no name

```rust no_run
# use snurr::{ProcessBuilder, Symbol};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.task("name or id", |input| {
    Symbol::Error.into()
});
# Ok(())
# }
```

##### Boundary with name

```rust no_run
# use snurr::{ProcessBuilder, Symbol};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.task("name or id", |input| {
    ("Not good", Symbol::Error).into()
});
# Ok(())
# }
```

## Gateways

Only branching/forking exclusive, event-based and inclusive gateways need to be added. If a gateway name is given then every gateway with same name and type will use the same closure. Register a gateway by **name** or by **id**, and return the outgoing sequence flow taken by **name** or **id**. No merging/joining gateway need to be added from the BPMN diagram with only one output.

Same gateway can do both join and fork instead of using two separate gateways. The latter is recommended for clarity. (i.e two gateways)

### Exclusive gateway

An exclusive gateway can select a flow named after the outgoing sequence flow.

#### One flow

```rust no_run
# use snurr::ProcessBuilder;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.exclusive("name or id", |input| {
    "YES".into()
});
# Ok(())
# }
```

#### Default flow

```rust no_run
# use snurr::ProcessBuilder;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.exclusive("name or id", |input| {
    Default::default()
});
# Ok(())
# }
```

### Event-based gateway

An event-based gateway can select a flow with an intermediate throw event, where the name and symbol must match those of the intermediate catching event. Event-based gateways require at least 2 outputs.

#### One flow

```rust no_run
# use snurr::{ProcessBuilder, Symbol};
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.event_based("name or id", |input| {
     ("Message", Symbol::Message).into()
});
# Ok(())
# }
```

### Inclusive gateway

An inclusive gateway can select one or many flows named after the outgoing sequence flow. A default flow should always be available in the BPMN diagram. Do not forget to merge the flows using a converging gateway. Only balanced gateway construction supported. See `Not Supported` section.

#### One flow

```rust no_run
# use snurr::ProcessBuilder;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.inclusive("name or id", |input| {
    "YES".into()
});
# Ok(())
# }
```

#### Many flows

```rust no_run
# use snurr::ProcessBuilder;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.inclusive("name or id", |input| {
    vec!["YES", "NO"].into()
});
# Ok(())
# }
```

#### Default flow

```rust no_run
# use snurr::ProcessBuilder;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.inclusive("name or id", |input| {
    Default::default()
});
# Ok(())
# }
```

### Parallel gateway

**Parallel gateways** run **all** available flows. No need to add gateway code. (And you can't). Only balanced gateway construction supported. See `Not Supported` section.

## Events

### End event

End events have different effects depending on where they are used. In a regular process or a subprocess. Some of these events are not used in accordance with the BPMN specification and are marked with `!BPMN`. They should not trigger a boundary event in this way.

- **Cancel** ends the transaction subprocess and run the cancel boundary.
- **Error** In a subprocess, ends and run the error boundary.
- **Escalation** In a subprocess, ends and run the Escalation boundary.
- **Signal** In a subprocess, ends and run the Signal boundary.
- **Terminate** ends the process. In a subprocess, ends and continues with the parent process.

#### Listen to end events

Optionally register an end callback to act on end events. If an error is returned it terminate the process prematurely and have it return the specified error. Only one can be registered.

```rust no_run
# use snurr::{ProcessBuilder, Symbol} ;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.end_event(|_input, name, symbol| {      
    match symbol {
        Symbol::Error => println!("act on an error, such as update the model or inform external systems"),
        _ => println!("ignore other end events"),
    }
    Ok(())
});
# Ok(())
# }
```

### Intermediate event

- Intermediate **none** events (no icon) don't do anything and just follow its output. 
- **Link** throw and catch need a matching name
- **Other symbols** call the optionally registered callback and just follow its output.

#### Listen to intermediate throw events

Optionally register an intermediate throw callback to act on throw events. If an error is returned it terminate the process prematurely and have it return the specified error. Only one can be registered.

```rust no_run
# use snurr::{ProcessBuilder, Symbol} ;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
#   ProcessBuilder::<()>::new("dummy.bpmn")?
.intermediate_throw_event(|_input, name, symbol| {      
    match symbol {
        Symbol::Message => println!("act on the message, for example by informing external systems"),
        _ => println!("ignore other throw events"),
    }
    Ok(())
});
# Ok(())
# }
```

 
### Boundary event

Only interrupting boundary events is implemented and can be used on a task or a subprocess.

Boundary symbols recognized:
- **Cancel** (Only transaction subprocess)
- **Compensation**
- **Conditional**
- **Error**
- **Escalation**
- **Message**
- **Signal**
- **Timer**

## Subprocess

Collapsed, expanded subprocess or transaction can be used.

## Not supported

### Process pools

Limited to one BPMN process per file (not to be confused with subprocesses, of which there can be several).

### Conditional Sequence Flows

Use an explicit gateway instead. Snurr return an `ParseError` if present.

### Unbalanced Inclusive or Parallel gateway construction

Re-write the process with balanced/symmetric gateway pairs. Snurr return an `RuntimeError` if occured while running the process. The check is only active in debug mode.
