use crate::{
    error::Error,
    model::{Event, Gateway, *},
    process::Diagram,
};
use std::collections::HashMap;

const BUILD_PROCESS_ERROR_MSG: &str = "couldn't build process";

#[derive(Default, Debug)]
pub struct ProcessData {
    // Start event in the process
    start: Option<usize>,
    data: Vec<Bpmn>,
}

impl ProcessData {
    fn add(&mut self, mut bpmn: Bpmn) -> Result<(), Error> {
        let len = self.data.len();
        if let Bpmn::Event(Event {
            event_type: EventType::Start,
            symbol: None,
            ..
        }) = bpmn
            && self.start.replace(len).is_some()
        {
            return Err(Error::BpmnRequirement(
                "Multiple start events with no symbol".into(),
            ));
        }

        bpmn.set_local_id(len);
        self.data.push(bpmn);
        Ok(())
    }

    // If it's a process or subprocess, we just insert it. Already correct ID.
    fn add_process(&mut self, bpmn: Bpmn) {
        self.data.push(bpmn);
    }

    pub fn data_mut(&mut self) -> &mut [Bpmn] {
        &mut self.data
    }

    pub fn data(&self) -> &[Bpmn] {
        &self.data
    }

    pub fn get(&self, index: usize) -> Option<&Bpmn> {
        self.data.get(index)
    }

    pub fn start(&self) -> Option<usize> {
        self.start
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bpmn> {
        self.data.iter()
    }
}

//
// data: [
//            [ // Might contain a sub process that has its data at index 1
//                Process 0 DATA
//            ],
//            [
//                Sub Process DATA
//            ],
//            [
//                Process 1 DATA
//            ],
//            [ // Definitions contains all top level processes. Always last.
//                Process 0, data at index 0
//                Process 1, data at index 2
//            ],
//        ]
//

#[derive(Default)]
pub(super) struct DataBuilder {
    data: Vec<ProcessData>,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
    process_stack: Vec<ProcessData>,
    stack: Vec<Bpmn>,
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
        if let Some(process_data) = self.process_stack.last_mut() {
            process_data.add(bpmn)?;
        }
        Ok(())
    }

    pub(super) fn update_symbol(&mut self, bpmn_type: &[u8]) {
        if let Some(Bpmn::Event(Event { symbol, .. })) = self.stack.last_mut() {
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
            text.replace(value);
        }
    }

    pub(super) fn end(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            check_unsupported(&bpmn)?;
            self.add_to_process(bpmn)?;
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), Error> {
        let Some((mut bpmn, mut process_data)) = self.stack.pop().zip(self.process_stack.pop())
        else {
            return Err(Error::Builder(BUILD_PROCESS_ERROR_MSG.into()));
        };

        // Process or sub process use local id to point to data index. Do NOT overwrite.
        bpmn.set_local_id(self.data.len());
        self.update_data(bpmn.id()?, process_data.data_mut());
        // Definitions collect all Processes
        // Processes collect all related sub processes
        if let Some(parent_process_data) = self.process_stack.last_mut() {
            parent_process_data.add_process(bpmn);
        }
        self.data.push(process_data);
        Ok(())
    }

    // Post process to fill local ids to some bpmn objects.
    fn update_data(&mut self, process_id: &Id, data: &mut [Bpmn]) {
        // Collect Bpmn id to index in array
        let bpmn_index: HashMap<String, usize> = data
            .iter()
            .enumerate()
            .filter_map(|(index, bpmn)| bpmn.id().ok().map(Id::bpmn).map(|id| (id.into(), index)))
            .collect();

        data.iter_mut().for_each(|bpmn| match bpmn {
            Bpmn::Activity { outputs, .. } => outputs.update_local_ids(&bpmn_index),
            Bpmn::Event(Event {
                event_type,
                id,
                outputs,
                attached_to_ref,
                symbol,
                name,
                ..
            }) => {
                outputs.update_local_ids(&bpmn_index);
                if let Some(attached_to_ref) = attached_to_ref {
                    attached_to_ref.update_local_id(&bpmn_index);

                    // Collect boundary to activity id
                    self.boundaries
                        .entry(attached_to_ref.bpmn().to_owned())
                        .or_default()
                        .push(*id.local());
                }

                if let Some(name) = name
                    && let Some(Symbol::Link) = symbol
                    && EventType::IntermediateCatch == *event_type
                {
                    self.catch_event_links
                        .entry(process_id.bpmn().into())
                        .or_default()
                        .insert(name.clone(), *id.local());
                }
            }
            Bpmn::Gateway(Gateway {
                default, outputs, ..
            }) => {
                outputs.update_local_ids(&bpmn_index);
                if let Some(default) = default {
                    default.update_local_id(&bpmn_index)
                }
            }
            Bpmn::SequenceFlow { target_ref, .. } => target_ref.update_local_id(&bpmn_index),
            _ => {}
        });
    }
}

impl From<DataBuilder> for Diagram {
    fn from(builder: DataBuilder) -> Self {
        Diagram::new(builder.data, builder.boundaries, builder.catch_event_links)
    }
}

fn check_unsupported(bpmn: &Bpmn) -> Result<(), Error> {
    Err(match bpmn {
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        Bpmn::SequenceFlow { id, name, .. } => Error::NotSupported(format!(
            "{}: {}",
            name.as_deref().unwrap_or(id.bpmn()),
            "conditional sequence flow",
        )),
        _ => return Ok(()),
    })
}
