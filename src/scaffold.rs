use std::{io::Write, path::Path};

use crate::{error::Error, model::Bpmn, Symbol};

#[derive(Debug)]
struct Gateway<'a> {
    bpmn: &'a Bpmn,
    output_names: Vec<&'a String>,
}

#[derive(Debug)]
struct Task<'a> {
    bpmn: &'a Bpmn,
    symbols: Vec<&'a Symbol>,
}

#[derive(Debug, Default)]
pub(crate) struct Scaffold<'a> {
    tasks: Vec<Task<'a>>,
    gateways: Vec<Gateway<'a>>,
}

impl<'a> Scaffold<'a> {
    pub(crate) fn add_task(&mut self, bpmn: &'a Bpmn, symbols: Vec<&'a Symbol>) {
        self.tasks.push(Task { bpmn, symbols });
    }

    pub(crate) fn add_gateway(&mut self, bpmn: &'a Bpmn, output_names: Vec<&'a String>) {
        self.gateways.push(Gateway { bpmn, output_names });
    }

    /// Generate code from all the task and gateways to the given file path.
    /// No file is allowed to exist at the target location.
    pub(crate) fn create(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut content = vec![];
        content.push("pub fn create_handler<T>() -> snurr::Eventhandler<T> {".into());
        content.push(
            "    let mut handler: snurr::Eventhandler<T> = snurr::Eventhandler::default();".into(),
        );

        // First all tasks
        for task in self.tasks.iter() {
            let Task {
                bpmn: Bpmn::Activity { id, name, .. },
                symbols,
            } = task
            else {
                continue;
            };

            let name_or_id = name.as_ref().unwrap_or(id);
            if !symbols.is_empty() {
                content.push(format!(
                    r#"    // "{}" boundary symbols: {:?}"#,
                    name_or_id, symbols
                ));
            }

            content.push(format!(
                r#"    handler.add_task("{}", |input| Ok(()));"#,
                name_or_id,
            ));
            content.push("".into());
        }

        // Second all gateways
        for gateway in self.gateways.iter() {
            let Gateway {
                bpmn: Bpmn::Gateway {
                    id, name, outputs, ..
                },
                output_names: out_names,
            } = gateway
            else {
                continue;
            };

            let name_or_id = name.as_ref().unwrap_or(id);
            content.push(format!(
                r#"    // "{}" output id(s) {:?}"#,
                name_or_id, outputs
            ));
            content.push(format!(
                r#"    handler.add_gateway("{}", |input| vec!{:?});"#,
                name_or_id, out_names
            ));
            content.push("".into());
        }

        content.push("    handler\n}".into());

        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;

        file.write_all(content.join("\n").as_bytes())?;
        Ok(())
    }
}
