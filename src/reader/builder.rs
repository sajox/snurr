use crate::{error::Error, model::*};
use std::collections::HashMap;

use super::ReaderResult;

#[derive(Default)]
pub(super) struct DataBuilder {
    data: HashMap<String, HashMap<String, Bpmn>>,
    process_stack: Vec<HashMap<String, Bpmn>>,
    stack: Vec<Bpmn>,
    definitions_id: Option<String>,
}

impl DataBuilder {
    pub(super) fn add(&mut self, bpmn: Bpmn) {
        self.stack.push(bpmn);
    }

    pub(super) fn add_new_process(&mut self, bpmn: Bpmn) {
        self.process_stack.push(Default::default());
        self.add(bpmn);
    }

    pub(super) fn add_to_process(&mut self, bpmn: Bpmn) -> Result<(), Error> {
        if let Some(process) = self.process_stack.last_mut() {
            process.insert(bpmn.id()?.into(), bpmn);
        }
        Ok(())
    }

    pub(super) fn update_symbol(&mut self, bpmn_type: &[u8]) {
        if let Some(Bpmn::Event { symbol, .. }) = self.stack.last_mut() {
            *symbol = bpmn_type.try_into().ok();
        }
    }

    pub(super) fn add_direction(&mut self, direction: &[u8]) {
        if let Some((
            Bpmn::Direction {
                text: Some(value), ..
            },
            parent,
        )) = self.stack.pop().zip(self.stack.last_mut())
        {
            match direction {
                OUTGOING => parent.add_output(value),
                _ => parent.add_input(value),
            }
        }
    }

    pub(super) fn update_start_id(&mut self) -> Result<(), Error> {
        if let Some((
            bpmn,
            Bpmn::Process { start_id, .. }
            | Bpmn::Activity {
                aktivity: ActivityType::SubProcess,
                start_id,
                ..
            },
        )) = self.stack.pop().zip(self.stack.last_mut())
        {
            // Update parent process or subprocess start_id
            *start_id = Some(bpmn.id()?.into());
            self.add_to_process(bpmn)?
        }
        Ok(())
    }

    pub(super) fn add_text(&mut self, value: String) -> Result<(), Error> {
        if let Some(Bpmn::Direction { text, .. }) = self.stack.last_mut() {
            *text = Some(value);
        }
        Ok(())
    }

    pub(super) fn end(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            check_unsupported(&bpmn)?;
            self.add_to_process(bpmn)?
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            if let Some(process) = self.process_stack.pop() {
                let id = bpmn.id()?.to_string();
                // Put the Bpmn model in parent scope and in 'data' it's related process data.
                // Definitions collect all Processes
                // Process collect all related sub processes
                if let Some(parent_process) = self.process_stack.last_mut() {
                    parent_process.insert(id.clone(), bpmn);
                } else {
                    // No parent, must be Definitions.
                    self.definitions_id = Some(id.clone());
                }
                self.data.insert(id, process);
            }
        }
        Ok(())
    }

    pub(super) fn finish(self) -> ReaderResult {
        Ok((
            self.definitions_id.ok_or(Error::MissingDefinitionsId)?,
            self.data,
        ))
    }
}

fn check_unsupported(bpmn: &Bpmn) -> Result<(), Error> {
    Err(match bpmn {
        Bpmn::Gateway {
            gateway: GatewayType::Parallel | GatewayType::Inclusive,
            id,
            name,
            outputs,
            inputs,
            ..
        } if outputs.len() > 1 && inputs.len() > 1 => Error::NotSupported(format!(
            "{}: {}",
            name.as_ref().unwrap_or(id),
            "both join and fork",
        )),
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        Bpmn::SequenceFlow { id, name, .. } => Error::NotSupported(format!(
            "{}: {}",
            name.as_ref().unwrap_or(id),
            "conditional sequence flow",
        )),
        _ => return Ok(()),
    })
}
