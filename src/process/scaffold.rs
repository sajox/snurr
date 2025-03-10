use std::{collections::HashSet, io::Write, path::Path};

use crate::{
    Process,
    error::Error,
    model::{ActivityType, Bpmn, GatewayType, Symbol},
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
        self.data.values().for_each(|process: &Vec<Bpmn>| {
            process.iter().for_each(|bpmn| {
                if let Bpmn::Activity {
                    activity: ActivityType::Task,
                    id,
                    ..
                } = bpmn
                {
                    let symbols = if let Some(boundaries) = self.boundaries.get(id) {
                        boundaries
                            .iter()
                            .filter_map(|index| process.get(*index))
                            .filter_map(|bpmn| {
                                if let Bpmn::Event {
                                    symbol: Some(symbol),
                                    name,
                                    ..
                                } = bpmn
                                {
                                    Some((name, symbol))
                                } else {
                                    None
                                }
                            })
                            .collect()
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
                        let names = outputs
                            .ids()
                            .iter()
                            .map(|index| process.get(**index))
                            .filter_map(|bpmn| {
                                if let Some(Bpmn::SequenceFlow { name, .. }) = bpmn {
                                    return name.as_ref();
                                }
                                None
                            })
                            .collect();
                        scaffold.add_gateway(bpmn, names);
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
    names: Vec<&'a String>,
}

#[derive(Debug)]
struct Task<'a> {
    bpmn: &'a Bpmn,
    symbols: Vec<(&'a Option<String>, &'a Symbol)>,
}

#[derive(Debug, Default)]
struct Scaffold<'a> {
    tasks: Vec<Task<'a>>,
    gateways: Vec<Gateway<'a>>,
}

impl<'a> Scaffold<'a> {
    fn add_task(&mut self, bpmn: &'a Bpmn, symbols: Vec<(&'a Option<String>, &'a Symbol)>) {
        self.tasks.push(Task { bpmn, symbols });
    }

    fn add_gateway(&mut self, bpmn: &'a Bpmn, names: Vec<&'a String>) {
        self.gateways.push(Gateway { bpmn, names });
    }

    // Generate code from all the task and gateways to the given file path.
    // No file is allowed to exist at the target location.
    fn create(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut content = vec![];
        content.push(
            "// Replace the '()' in the Eventhandler<()> return type with your own type.".into(),
        );
        content.push("pub fn create_handler() -> snurr::Eventhandler<()> {".into());
        content.push("    let mut handler = snurr::Eventhandler::default();".into());

        // Do not generate duplicates
        let mut seen_tasks: HashSet<&String> = HashSet::new();
        let mut seen_gateways: HashSet<&String> = HashSet::new();

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
            if seen_tasks.insert(name_or_id) {
                if !symbols.is_empty() {
                    content.push(format!(
                        r#"    // "{}" boundary symbols: {:?}"#,
                        name_or_id, symbols
                    ));
                }

                content.push(format!(
                    r#"    handler.add_task("{}", |input| None);"#,
                    name_or_id,
                ));
                content.push("".into());
            }
        }

        // Second all gateways
        for gateway in self.gateways.iter() {
            let Gateway {
                bpmn:
                    Bpmn::Gateway {
                        gateway,
                        id,
                        name,
                        outputs,
                        ..
                    },
                names,
            } = gateway
            else {
                continue;
            };

            let name_or_id = name.as_ref().unwrap_or(id);
            if seen_gateways.insert(name_or_id) {
                content.push(format!(
                    r#"    // {} gateway. Names: {}. Flows: {}."#,
                    gateway,
                    names
                        .iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    outputs
                ));
                content.push(format!(
                    r#"    handler.add_gateway("{}", |input| Default::default());"#,
                    name_or_id,
                ));
                content.push("".into());
            }
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
