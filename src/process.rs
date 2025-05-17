mod engine;
pub mod handler;
mod reader;
mod scaffold;

use engine::ExecuteData;
use handler::{Data, Handler, TaskResult};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::{
    IntermediateEvent, With,
    error::Error,
    model::{ActivityType, Bpmn, BpmnLocal, Gateway, GatewayType},
};

#[derive(Debug)]
pub struct Diagram {
    data: HashMap<String, Vec<Bpmn>>,
    definitions_id: String,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
}

impl Diagram {
    pub fn install_task_function(&mut self, match_value: &str, index: usize) {
        for bpmn in self.data.values_mut().flatten() {
            if let Bpmn::Activity {
                id, func_idx, name, ..
            } = bpmn
            {
                if Diagram::match_name_or_id(name.as_deref(), id, match_value) {
                    *func_idx = Some(index)
                }
            }
        }
    }

    pub fn install_gateway_function(
        &mut self,
        gw_type: GatewayType,
        match_value: &str,
        index: usize,
    ) {
        for bpmn in self.data.values_mut().flatten() {
            if let Bpmn::Gateway(Gateway {
                gateway,
                id: BpmnLocal(id, _),
                func_idx,
                name,
                ..
            }) = bpmn
            {
                if Diagram::match_name_or_id(name.as_deref(), id, match_value)
                    && gw_type == *gateway
                {
                    *func_idx = Some(index)
                }
            }
        }
    }

    pub fn find_missing_functions(&self) -> HashSet<String> {
        self.data
            .values()
            .flatten()
            .fold(HashSet::new(), |mut acc, bpmn| {
                acc.insert(match bpmn {
                    Bpmn::Activity {
                        id,
                        name,
                        func_idx: None,
                        activity:
                            activity @ (ActivityType::Task
                            | ActivityType::ScriptTask
                            | ActivityType::UserTask
                            | ActivityType::ServiceTask
                            | ActivityType::CallActivity
                            | ActivityType::ReceiveTask
                            | ActivityType::SendTask
                            | ActivityType::ManualTask
                            | ActivityType::BusinessRuleTask),
                        ..
                    } => format!("{}: {}", activity, name.as_ref().unwrap_or(id)),
                    Bpmn::Gateway(Gateway {
                        gateway:
                            gateway @ (GatewayType::EventBased
                            | GatewayType::Exclusive
                            | GatewayType::Inclusive),
                        name,
                        id: BpmnLocal(id, _),
                        func_idx: None,
                        outputs,
                        ..
                    }) if outputs.len() > 1 => {
                        format!("{}: {}", gateway, name.as_ref().unwrap_or(id))
                    }
                    _ => {
                        return acc;
                    }
                });
                acc
            })
    }

    fn match_name_or_id(name: Option<&str>, id: &str, value: &str) -> bool {
        name.map(|name| name == value).unwrap_or(false) || id == value
    }
}

/// Process that contains information from the BPMN file
pub struct Process<S, T> {
    diagram: Diagram,
    handler: Handler<T>,
    _marker: PhantomData<S>,
}

pub struct Build;
pub struct Run;

impl<T> Process<Build, T> {
    /// Create new process and initialize it from the BPMN file path.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<Build, ()> = Process::new("examples/example.bpmn")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            diagram: reader::read_bpmn(quick_xml::Reader::from_file(path)?)?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }

    pub fn task<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        let index = self.handler.add_task(func);
        self.diagram.install_task_function(name.as_ref(), index);
        self
    }

    pub fn exclusive<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync,
    {
        let index = self.handler.add_exclusive(func);
        self.diagram
            .install_gateway_function(GatewayType::Exclusive, name.as_ref(), index);
        self
    }

    pub fn inclusive<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        let index = self.handler.add_inclusive(func);
        self.diagram
            .install_gateway_function(GatewayType::Inclusive, name.as_ref(), index);
        self
    }

    pub fn event_based<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> IntermediateEvent + 'static + Sync,
    {
        let index = self.handler.add_event_based(func);
        self.diagram
            .install_gateway_function(GatewayType::EventBased, name.as_ref(), index);
        self
    }

    pub fn build(self) -> Result<Process<Run, T>, Error> {
        let result = self.diagram.find_missing_functions();
        if result.is_empty() {
            Ok(Process {
                diagram: self.diagram,
                handler: self.handler,
                _marker: Default::default(),
            })
        } else {
            Err(Error::MissingImplementations(
                result.into_iter().collect::<Vec<_>>().join(", "),
            ))
        }
    }
}

impl<T> FromStr for Process<Build, T> {
    type Err = Error;

    /// Create new process and initialize it from a BPMN `&str`.
    /// ```
    /// use snurr::{Build, Process};
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/example.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<Build, ()> = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            diagram: reader::read_bpmn(quick_xml::Reader::from_str(s))?,
            handler: Default::default(),
            _marker: Default::default(),
        })
    }
}

impl<T> Process<Run, T> {
    /// Run the process and return the `ProcessResult` or an `Error`.
    /// ```
    /// use snurr::Process;
    ///
    /// #[derive(Debug, Default)]
    /// struct Counter {
    ///     count: u32,
    /// }
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     pretty_env_logger::init();
    ///
    ///     // Create process from BPMN file
    ///     let bpmn = Process::<_, Counter>::new("examples/example.bpmn")?
    ///         .task("Count 1", |input| {
    ///             input.lock().unwrap().count += 1;
    ///             None
    ///         })
    ///         .exclusive("equal to 3", |input| {
    ///             let result = if input.lock().unwrap().count == 3 {
    ///                 "YES"
    ///             } else {
    ///                 "NO"
    ///             };
    ///             result.into()
    ///         })
    ///         .build()?;
    ///
    ///     // Run the process with handler and data
    ///     let result = bpmn.run(Counter::default())?;
    ///
    ///     // Print the result.
    ///     println!("Result: {:?}", result);
    ///     Ok(())
    /// }
    /// ```
    pub fn run(&self, data: T) -> Result<T, Error>
    where
        T: Send,
    {
        let data = Arc::new(Mutex::new(data));

        // Run every process specified in the diagram
        for bpmn in self
            .diagram
            .data
            .get(&self.diagram.definitions_id)
            .ok_or(Error::MissingDefinitionsId)?
            .iter()
        {
            if let Bpmn::Process { id, .. } = bpmn {
                let process_data = self
                    .diagram
                    .data
                    .get(id)
                    .ok_or_else(|| Error::MissingProcessData(id.into()))?;

                self.execute(&ExecuteData::new(process_data, id, Arc::clone(&data)))?;
            }
        }

        Arc::into_inner(data)
            .ok_or(Error::NoProcessResult)?
            .into_inner()
            .map_err(|_| Error::NoProcessResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?
            .task("Count 1", |_| None)
            .exclusive("equal to 3", |_| None)
            .build()?;
        bpmn.run({})?;
        Ok(())
    }
}
