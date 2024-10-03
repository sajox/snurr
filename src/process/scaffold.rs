use std::{collections::HashMap, io::Write, path::Path};

use crate::{
    error::Error,
    model::{ActivityType, Bpmn, GatewayType, Symbol},
    Process,
};

impl Process {
    /// Generate code from all the task and gateways to the given file path.
    /// No file with same name is allowed to exist at the target location.
    /// ```
    /// use snurr::Process;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn = Process::new("examples/example.bpmn")?;
    ///     bpmn.scaffold("examples/scaffold.rs")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn scaffold(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut scaffold = Scaffold::default();
        self.data
            .values()
            .for_each(|process: &HashMap<String, Bpmn>| {
                process.values().for_each(|bpmn| {
                    if let Bpmn::Activity {
                        activity: ActivityType::Task,
                        id,
                        ..
                    } = bpmn
                    {
                        let symbols = if let Some(map) = self.activity_ids.get(id) {
                            map.keys().collect::<Vec<&Symbol>>()
                        } else {
                            Vec::new()
                        };
                        scaffold.add_task(bpmn, symbols);
                    }

                    if let Bpmn::Gateway {
                        gateway: GatewayType::Exclusive | GatewayType::Inclusive,
                        outputs,
                        ..
                    } = bpmn
                    {
                        if outputs.len() > 1 {
                            scaffold.add_gateway(bpmn);
                        }
                    }
                });
            });

        scaffold.create(path)
    }
}

#[derive(Debug)]
struct Gateway<'a> {
    bpmn: &'a Bpmn,
}

#[derive(Debug)]
struct Task<'a> {
    bpmn: &'a Bpmn,
    symbols: Vec<&'a Symbol>,
}

#[derive(Debug, Default)]
struct Scaffold<'a> {
    tasks: Vec<Task<'a>>,
    gateways: Vec<Gateway<'a>>,
}

impl<'a> Scaffold<'a> {
    fn add_task(&mut self, bpmn: &'a Bpmn, symbols: Vec<&'a Symbol>) {
        self.tasks.push(Task { bpmn, symbols });
    }

    fn add_gateway(&mut self, bpmn: &'a Bpmn) {
        self.gateways.push(Gateway { bpmn });
    }

    /// Generate code from all the task and gateways to the given file path.
    /// No file is allowed to exist at the target location.
    fn create(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut content = vec![];
        content.push(
            "// Replace the '()' in the Eventhandler<()> return type with your own type.".into(),
        );
        content.push("pub fn create_handler() -> snurr::Eventhandler<()> {".into());
        content.push("    let mut handler = snurr::Eventhandler::default();".into());

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
            } = gateway
            else {
                continue;
            };

            let name_or_id = name.as_ref().unwrap_or(id);
            content.push(format!(r#"    // "{}", {:?}"#, name_or_id, outputs));
            content.push(format!(
                r#"    handler.add_gateway("{}", |input| vec!{:?});"#,
                name_or_id,
                outputs.names()
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
