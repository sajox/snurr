# Snurr

**Snurr** is a lightweight workflow engine that can run the process flow from a Business Process Model and Notation (BPMN) 2.0 file created by <https://demo.bpmn.io/new> or the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.

How to:

1. Create your BPMN diagram.
2. Scaffold the initial BPMN diagram so you don't have to do the boilerplate code.
3. Add custom behavior using [Rust](https://rust-lang.org) code from a small API. The wiring is already setup from the file.
4. Run your process in single or multi-threaded mode.

Maintainability:

- Update the BPMN diagram with new activities and gateways to meet changing requirements. The code is loosely coupled.
- The BPMN file is the actual design. Forget outdated documentation.
- No complicated configuration or database requirements.

This is not intended to be a full-fledged BPMN 2.0 solution, but rather a solution that is easy to embed and use. Read the documentation on what is supported.

## Example

### BPMN diagram

![image of counter.bpmn](https://github.com/sajox/snurr/blob/main/assets/images/example.png?raw=true)

### Usage

```toml
[dependencies]
snurr = { git = "https://github.com/sajox/snurr.git" }
log = "0.4"
pretty_env_logger = "0.5"
```

```rust
use pretty_env_logger;
use snurr::Process;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

#[derive(Debug, Default)]
struct Counter(AtomicU32);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();

    let bpmn = Process::<Counter>::new("examples/counter.bpmn")?
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
    println!("{result:?}");
    Ok(())
}
```

### Output

If `RUST_LOG=debug` is set when running example

```text
 DEBUG snurr::process::engine > Start `Begin process`
 DEBUG snurr::process::engine > SequenceFlow `count`
 DEBUG snurr::process::engine > Task `Count 1`
 DEBUG snurr::process::engine > SequenceFlow `control`
 DEBUG snurr::process::engine > Exclusive `equal to 3`
 DEBUG snurr::process::engine > SequenceFlow `NO`
 DEBUG snurr::process::engine > Task `Count 1`
 DEBUG snurr::process::engine > SequenceFlow `control`
 DEBUG snurr::process::engine > Exclusive `equal to 3`
 DEBUG snurr::process::engine > SequenceFlow `NO`
 DEBUG snurr::process::engine > Task `Count 1`
 DEBUG snurr::process::engine > SequenceFlow `control`
 DEBUG snurr::process::engine > Exclusive `equal to 3`
 DEBUG snurr::process::engine > SequenceFlow `YES`
 DEBUG snurr::process::engine > End `End process`
Counter(3)
```