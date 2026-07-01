# Snurr

**Snurr** can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new> or the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.

- Add your own behavior with Rust code from a small API. The wiring is already setup from the file.
- Easy to update the BPMN diagram with new Task and Gateways without the need to refactor your old code.
- The BPMN file is the actual design. Forget outdated documentation.
- Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
- Contains no database.
- Single or multithreaded (opt in)

## Example

BPMN diagram used in example.

![BPMN example](https://github.com/sajox/snurr/blob/main/assets/images/example.png?raw=true)

### Usage

```toml
[dependencies]
snurr = { git = "https://github.com/sajox/snurr.git" }
log = "0.4"
pretty_env_logger = "0.5"
```

```rust
use snurr::Process;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
extern crate pretty_env_logger;

#[derive(Debug, Default)]
struct Counter {
    count: AtomicU32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let bpmn = Process::<Counter>::new("examples/example.bpmn")?
        .task("Count 1", |input| {
            input.count.fetch_add(1, Relaxed);
            Default::default()
        })
        .exclusive("equal to 3", |input| {
            match input.count.load(Relaxed) {
                3 => "YES",
                _ => "NO",
            }
            .into()
        })
        .build()?;

    let result = bpmn.run(Default::default())?;
    println!("Count: {}", result.count.load(Relaxed));
    Ok(())
}
```

### Output

If RUST_LOG=info is set when running [example](#usage)

```
 INFO  snurr::process::engine > Start "Begin process"
 INFO  snurr::process::engine > SequenceFlow "count"
 INFO  snurr::process::engine > Task "Count 1"
 INFO  snurr::process::engine > SequenceFlow "control"
 INFO  snurr::process::engine > Exclusive "equal to 3"
 INFO  snurr::process::engine > SequenceFlow "NO"
 INFO  snurr::process::engine > Task "Count 1"
 INFO  snurr::process::engine > SequenceFlow "control"
 INFO  snurr::process::engine > Exclusive "equal to 3"
 INFO  snurr::process::engine > SequenceFlow "NO"
 INFO  snurr::process::engine > Task "Count 1"
 INFO  snurr::process::engine > SequenceFlow "control"
 INFO  snurr::process::engine > Exclusive "equal to 3"
 INFO  snurr::process::engine > SequenceFlow "YES"
 INFO  snurr::process::engine > End "End process"
Count: 3
```

### Prepared sample

Run or copy the simple.rs in the examples folder

```
RUST_LOG=info cargo run --example simple
```
