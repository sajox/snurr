# Snurr

[![Build Status](https://github.com/sajox/snurr/actions/workflows/rust.yml/badge.svg)](https://github.com/sajox/snurr/actions)

Create and run the process flow from a BPMN 2.0 file created by [BPMN Editor demo](https://demo.bpmn.io/new). Add your own behavior with Rust code from a small API. Read the [Snurr documentation](https://github.com/sajox/snurr/blob/main/docs/documentation.md) and explore the **tests** folder for more examples. 

**_NOTE:_** To view or edit BPMN files in your project you can use the [BPMN Editor](https://github.com/bpmn-io/vs-code-bpmn-io) plugin in VS Code.   

![Tasks](https://github.com/sajox/snurr/blob/main/assets/images/vscode-plugin-bpmnio.png?raw=true)

## Example

BPMN diagram used in example.

![BPMN example](https://github.com/sajox/snurr/blob/main/assets/images/example.png?raw=true)

### Usage

```toml
[dependencies]
snurr = { git = "https://github.com/sajox/snurr.git"}
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

    let bpmn = Process::new("example.bpmn")?;
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
