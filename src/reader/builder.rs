use crate::{error::Error, model::*};
use std::collections::HashMap;

use super::ReaderResult;

#[derive(Default)]
struct ProcessData {
    process: HashMap<String, Bpmn>,
    sequence_flows: Vec<Bpmn>,
}

#[derive(Default)]
pub(super) struct DataBuilder {
    data: HashMap<String, HashMap<String, Bpmn>>,
    process_stack: Vec<ProcessData>,
    stack: Vec<Bpmn>,
    definitions_id: Option<String>,
}

impl DataBuilder {
    pub(super) fn add(&mut self, bpmn: Bpmn) {
        self.stack.push(bpmn);
    }

    pub(super) fn add_sequence_flow(&mut self, bpmn: Bpmn) {
        if let Some(current) = self.process_stack.last_mut() {
            current.sequence_flows.push(bpmn)
        }
    }

    pub(super) fn add_new_process(&mut self, bpmn: Bpmn) {
        self.process_stack.push(Default::default());
        self.add(bpmn);
    }

    pub(super) fn add_to_process(&mut self, bpmn: Bpmn) -> Result<(), Error> {
        if let Some(current) = self.process_stack.last_mut() {
            current.process.insert(bpmn.id()?.into(), bpmn);
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
            if let Some(mut current) = self.process_stack.pop() {
                self.register_gateway_names_and_add(current.sequence_flows, &mut current.process)?;

                let id = bpmn.id()?.to_string();
                // Put the Bpmn model in parent scope and in 'data' it's related process data.
                // Definitions collect all Processes
                // Process collect all related sub processes
                if let Some(parent) = self.process_stack.last_mut() {
                    parent.process.insert(id.clone(), bpmn);
                } else {
                    // No parent, must be Definitions.
                    self.definitions_id = Some(id.clone());
                }
                self.data.insert(id, current.process);
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

    // Register gateway names when current process finish. All data is then collected.
    fn register_gateway_names_and_add(
        &mut self,
        sequence_flows: Vec<Bpmn>,
        process: &mut HashMap<String, Bpmn>,
    ) -> Result<(), Error> {
        // Update gateway output names from sequence flow.
        for sequence_flow in sequence_flows {
            if let Bpmn::SequenceFlow {
                id,
                name: Some(name),
                source_ref,
                ..
            } = &sequence_flow
            {
                if let Some(Bpmn::Gateway { outputs, .. }) = process.get_mut(source_ref) {
                    outputs.register_name(id, name);
                }
            }

            // Insert sequence flow to current process.
            process.insert(sequence_flow.id()?.into(), sequence_flow);
        }
        Ok(())
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
