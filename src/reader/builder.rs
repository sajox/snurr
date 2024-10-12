use crate::{error::Error, model::*};
use std::collections::HashMap;

use super::ReaderResult;

#[derive(Default)]
pub(super) struct DataBuilder {
    data: HashMap<String, Vec<Bpmn>>,
    process_stack: Vec<Vec<Bpmn>>,
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

    pub(super) fn add_to_process(&mut self, bpmn: Bpmn) {
        if let Some(current) = self.process_stack.last_mut() {
            current.push(bpmn);
        }
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

    pub(super) fn add_text(&mut self, value: String) {
        if let Some(Bpmn::Direction { text, .. }) = self.stack.last_mut() {
            *text = Some(value);
        }
    }

    pub(super) fn end(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            check_unsupported(&bpmn)?;
            self.add_to_process(bpmn);
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            if let Some(mut current) = self.process_stack.pop() {
                self.register_data(&mut current);
                let id = bpmn.id()?.to_string();
                // Put the Bpmn model in parent scope and in 'data' it's related process data.
                // Definitions collect all Processes
                // Process collect all related sub processes
                if let Some(parent) = self.process_stack.last_mut() {
                    parent.push(bpmn);
                } else {
                    // No parent, must be Definitions.
                    self.definitions_id = Some(id.clone());
                }
                self.data.insert(id, current);
            }
        }

        Ok(())
    }

    fn register_data(&mut self, data: &mut [Bpmn]) {
        // Collect Bpmn id to index in array
        let bpmn_index: HashMap<String, usize> = data
            .iter()
            .enumerate()
            .filter_map(|(index, bpmn)| {
                let bpmn_id = match bpmn {
                    Bpmn::Activity { id, .. }
                    | Bpmn::Event {
                        id: BpmnLocal(id, _),
                        ..
                    }
                    | Bpmn::Gateway { id, .. }
                    | Bpmn::SequenceFlow { id, .. } => id.to_string(),
                    _ => return None,
                };
                Some((bpmn_id, index))
            })
            .collect();

        // Update information
        data.iter_mut().for_each(|bpmn| match bpmn {
            Bpmn::Activity { outputs, .. } => outputs.update_local_ids(&bpmn_index),
            Bpmn::Event {
                id,
                outputs,
                attached_to_ref,
                ..
            } => {
                outputs.update_local_ids(&bpmn_index);
                update_local_id(id, &bpmn_index);
                if let Some(attached_to_ref) = attached_to_ref {
                    update_local_id(attached_to_ref, &bpmn_index);
                }
            }
            Bpmn::Gateway {
                default, outputs, ..
            } => {
                outputs.update_local_ids(&bpmn_index);
                if let Some(default) = default {
                    update_local_id(default, &bpmn_index);
                }
            }
            Bpmn::SequenceFlow { target_ref, .. } => {
                update_local_id(target_ref, &bpmn_index);
            }
            _ => {}
        });
    }

    pub(super) fn finish(self) -> ReaderResult {
        Ok((
            self.definitions_id.ok_or(Error::MissingDefinitionsId)?,
            self.data,
        ))
    }
}

fn update_local_id(BpmnLocal(bid, lid): &mut BpmnLocal, map: &HashMap<String, usize>) {
    if let Some(index) = map.get(bid) {
        *lid = *index;
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
