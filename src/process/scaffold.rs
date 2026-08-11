use std::{collections::HashSet, io::Write, path::Path};

use crate::{
    Process,
    bpmn::{Activity, ActivityType, Bpmn, Gateway, GatewayType, Symbol},
};

impl<T> Process<T> {
    /// Generate code from all the task and gateways to the given file path.
    /// No file with same name is allowed to exist at the target location.
    /// ```no_run
    /// use snurr::Process;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = Process::new("examples/counter.bpmn")?;
    ///     bpmn.scaffold("examples/scaffold.rs")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn scaffold(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let mut scaffold = Scaffold::default();
        for process in self.diagram.data() {
            let mut boundaries = process.events.boundaries();
            for bpmn in process.iter() {
                match bpmn {
                    Bpmn::Activity(Activity {
                        activity_type: ActivityType::Task,
                        id,
                        ..
                    }) => {
                        scaffold.add_task(bpmn, boundaries.remove(id.local()).unwrap_or_default());
                    }
                    Bpmn::Gateway(
                        gateway @ Gateway {
                            gateway_type:
                                GatewayType::Exclusive
                                | GatewayType::Inclusive
                                | GatewayType::EventBased,
                            outputs,
                            ..
                        },
                    ) if outputs.len() > 1 => {
                        let names = outputs
                            .iter()
                            .filter_map(|index| process.get(*index))
                            .filter_map(|bpmn| {
                                if let Bpmn::SequenceFlow { name, .. } = bpmn {
                                    return name.as_ref();
                                }
                                None
                            })
                            .collect();
                        scaffold.add_gateway(gateway, names);
                    }
                    _ => {}
                }
            }
        }
        scaffold.create(path)
    }
}

#[derive(Debug)]
struct GatewayInner<'a> {
    gateway: &'a Gateway,
    names: Vec<&'a String>,
}

#[derive(Debug)]
struct Task<'a> {
    bpmn: &'a Bpmn,
    symbols: Vec<(Option<String>, Symbol)>,
}

#[derive(Debug, Default)]
struct Scaffold<'a> {
    tasks: Vec<Task<'a>>,
    gateways: Vec<GatewayInner<'a>>,
}

impl<'a> Scaffold<'a> {
    fn add_task(&mut self, bpmn: &'a Bpmn, symbols: Vec<(Option<String>, Symbol)>) {
        self.tasks.push(Task { bpmn, symbols });
    }

    fn add_gateway(&mut self, gateway: &'a Gateway, names: Vec<&'a String>) {
        self.gateways.push(GatewayInner { gateway, names });
    }

    // Generate code from all the task and gateways to the given file path.
    // No file is allowed to exist at the target location.
    fn create(&mut self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let mut content = vec![
            "use snurr::{Process, Run, error::BuildError};\n".into(),
            "// Replace () with your type".into(),
            "pub fn build(process: Process<()>) -> Result<Process<(), Run>, BuildError> {".into(),
            r#"  process"#.into(),
        ];

        // Do not generate duplicates
        let mut seen_tasks: HashSet<&str> = HashSet::new();
        let mut seen_gateways: HashSet<&str> = HashSet::new();

        // First all tasks
        for task in self.tasks.iter() {
            let Task {
                bpmn: Bpmn::Activity(Activity { id, name, .. }),
                symbols,
            } = task
            else {
                continue;
            };

            let name_or_id = name.as_deref().unwrap_or(id.bpmn());
            if seen_tasks.insert(name_or_id) {
                if !symbols.is_empty() {
                    content.push(format!(
                        r#"    // "{name_or_id}" boundary symbols: {symbols:?}"#
                    ));
                }

                content.push(format!(
                    r#"    .task("{name_or_id}", |input| Default::default())"#
                ));
                content.push("".into());
            }
        }

        // Second all gateways
        for GatewayInner {
            gateway:
                Gateway {
                    gateway_type,
                    id,
                    name,
                    outputs,
                    ..
                },
            names,
        } in self.gateways.iter()
        {
            let name_or_id = name.as_deref().unwrap_or(id.bpmn());
            if seen_gateways.insert(name_or_id) {
                content.push(format!(
                    r#"    // Names: {}. Ids: {}."#,
                    names
                        .iter()
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                    outputs
                ));

                match gateway_type {
                    GatewayType::Exclusive => content.push(format!(
                        r#"    .exclusive("{name_or_id}", |input| Default::default())"#,
                    )),
                    GatewayType::Inclusive => content.push(format!(
                        r#"    .inclusive("{name_or_id}", |input| Default::default())"#,
                    )),
                    GatewayType::EventBased => content.push(format!(
                        r#"    .event_based("{name_or_id}", |input| // Implement)"#,
                    )),
                    _ => {}
                }

                content.push("".into());
            }
        }
        content.push("    .build()\n}".into());

        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;

        file.write_all(content.join("\n").as_bytes())?;
        Ok(())
    }
}
