mod engine;
pub mod handler;
mod reader;
mod scaffold;

use engine::ExecuteData;
use handler::{Data, Handler, TaskResult};
use std::{
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::{
    Symbol, With,
    error::Error,
    model::{Bpmn, BpmnLocal, Gateway, HashMap},
};

#[derive(Debug)]
pub struct Diagram {
    data: HashMap<String, Vec<Bpmn>>,
    definitions_id: String,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
}

impl Diagram {
    pub fn update_func_id_task(&mut self, index: usize, match_name: &str) {
        self.data.iter_mut().for_each(|process| {
            process.1.iter_mut().for_each(|bpmn| match bpmn {
                Bpmn::Activity {
                    id, func_id, name, ..
                } if name
                    .as_ref()
                    .map(|v| v.as_str() == match_name)
                    .unwrap_or(false)
                    || id.as_str() == match_name =>
                {
                    *func_id = Some(index);
                }
                _ => {}
            })
        });
    }

    pub fn update_func_id_gw(&mut self, index: usize, match_name: &str) {
        self.data.iter_mut().for_each(|process| {
            process.1.iter_mut().for_each(|bpmn| match bpmn {
                Bpmn::Gateway(Gateway {
                    id: BpmnLocal(bid, _),
                    func_id,
                    name,
                    ..
                }) if name
                    .as_ref()
                    .map(|v| v.as_str() == match_name)
                    .unwrap_or(false)
                    || bid.as_str() == match_name =>
                {
                    *func_id = Some(index)
                }
                _ => {}
            })
        });
    }
}

/// Process that contains information from the BPMN file
pub struct Process<T> {
    diagram: Diagram,
    handler: Handler<T>,
}

impl<T> Process<T> {
    /// Create new process and initialize it from the BPMN file path.
    /// ```
    /// use snurr::Process;
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = Process::new("examples/example.bpmn")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            diagram: reader::read_bpmn(quick_xml::Reader::from_file(path)?)?,
            handler: Default::default(),
        })
    }

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
    ///     let bpmn = Process::new("examples/example.bpmn")?;
    ///     // Register Task and Gateways...
    ///     let pr = bpmn.run(Counter::default())?;
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

    pub fn task<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> TaskResult + 'static + Sync,
    {
        let index = self.handler.add_task(func);
        self.diagram.update_func_id_task(index, name.as_ref());
        self
    }

    pub fn exclusive<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> Option<&'static str> + 'static + Sync,
    {
        let index = self.handler.add_exclusive(func);
        self.diagram.update_func_id_gw(index, name.as_ref());
        self
    }

    pub fn inclusive<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> With + 'static + Sync,
    {
        let index = self.handler.add_inclusive(func);
        self.diagram.update_func_id_gw(index, name.as_ref());
        self
    }

    pub fn event_based<F>(mut self, name: impl AsRef<str>, func: F) -> Self
    where
        F: Fn(Data<T>) -> (Option<&'static str>, Symbol) + 'static + Sync,
    {
        let index = self.handler.add_event_based(func);
        self.diagram.update_func_id_gw(index, name.as_ref());
        self
    }
}

impl<T> FromStr for Process<T> {
    type Err = Error;

    /// Create new process and initialize it from a BPMN `&str`.
    /// ```
    /// use snurr::Process;
    ///
    /// static BPMN_DATA: &str = include_str!("../examples/example.bpmn");
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let bpmn: Process<()> = BPMN_DATA.parse()?;
    ///     Ok(())
    /// }
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            diagram: reader::read_bpmn(quick_xml::Reader::from_str(s))?,
            handler: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_run() -> Result<(), Box<dyn std::error::Error>> {
        let bpmn = Process::new("examples/example.bpmn")?;
        bpmn.run({})?;
        Ok(())
    }
}
