use std::collections::HashMap;

use crate::{
    bpmn::{Event, *},
    diagram::{Diagram, ProcessData, events::Events},
    process::{ParseError, ParseErrorKind},
};

//
// definition: [ // Definitions contains all top level processes.
//                Process 0, data at index 0
//                Process 1, data at index 2
//             ]
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
//        ]
//

#[derive(Default)]
pub(super) struct DataBuilder {
    // Top level processes collected in definitions
    definitions: Vec<Bpmn>,

    // Process and subprocess data
    data: Vec<ProcessData>,
    process_stack: Vec<ProcessConstruction>,
    stack: Vec<Bpmn>,

    // Temporary text from XML
    text: Option<String>,
}

impl DataBuilder {
    pub(super) fn add(&mut self, bpmn: Bpmn) {
        self.stack.push(bpmn);
    }

    pub(super) fn add_new_process(&mut self, bpmn: Bpmn) {
        self.process_stack.push(Default::default());
        self.add(bpmn);
    }

    pub(super) fn add_to_process(&mut self, bpmn: Bpmn) -> Result<(), ParseError> {
        if let Some(process_data) = self.process_stack.last_mut() {
            process_data.add(bpmn)?;
        }
        Ok(())
    }

    pub(super) fn update_symbol(&mut self, bpmn_type: &str) {
        if let Some(Bpmn::Event(Event { symbol, .. })) = self.stack.last_mut() {
            *symbol = bpmn_type.try_into().ok();
        }
    }

    pub(super) fn add_text_to_parent(&mut self, bpmn_type: &str) {
        if let Some(parent) = self.stack.last_mut()
            && let Some(text) = self.text.take()
        {
            match bpmn_type {
                OUTGOING => parent.add_output(text),
                _ => parent.add_input(),
            }
        }
    }

    pub(super) fn add_text(&mut self, value: impl Into<String>) {
        self.text.replace(value.into());
    }

    pub(super) fn end(&mut self) -> Result<(), ParseError> {
        if let Some(bpmn) = self.stack.pop() {
            check_unsupported(&bpmn)?;
            self.add_to_process(bpmn)?;
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), ParseError> {
        let Some((mut bpmn, mut process_data)) = self.stack.pop().zip(self.process_stack.pop())
        else {
            Err(ParseErrorKind::ProcessBuild)?
        };

        // Process or sub process use index to point to data.
        bpmn.update_data_index(self.data.len());

        match self.process_stack.last_mut() {
            // Processes collect all related subprocesses
            Some(parent_process_data) => parent_process_data.add(bpmn)?,
            // Definitions collect all processes
            None => self.definitions.push(bpmn),
        }

        process_data.finalize();
        self.data.push(process_data.try_into()?);
        Ok(())
    }
}

impl From<DataBuilder> for Diagram {
    fn from(builder: DataBuilder) -> Self {
        Diagram::new(
            builder.definitions.into_boxed_slice(),
            builder.data.into_boxed_slice(),
        )
    }
}

fn check_unsupported(bpmn: &Bpmn) -> Result<(), ParseError> {
    Err(match bpmn {
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        Bpmn::SequenceFlow { id, name, .. } => ParseErrorKind::NotSupported(format!(
            "{}: {}",
            name.as_deref().unwrap_or(id.bpmn()),
            "conditional sequence flow",
        )),
        _ => return Ok(()),
    })?
}

#[derive(Default, Debug)]
struct ProcessConstruction {
    start: Option<usize>,
    data: Vec<Bpmn>,
    events: Events,
}

impl ProcessConstruction {
    fn add(&mut self, mut bpmn: Bpmn) -> Result<(), ParseError> {
        let len = self.data.len();
        if let Bpmn::Event(Event {
            event_type: EventType::Start,
            symbol: None,
            ..
        }) = bpmn
            && self.start.replace(len).is_some()
        {
            Err(ParseErrorKind::NotSupported("multiple start event".into()))?
        }

        bpmn.update_local_id(len);
        self.data.push(bpmn);
        Ok(())
    }

    // Everything in the process has been collected. Update local IDs with correct index.
    fn finalize(&mut self) {
        // Collect Bpmn id to index in array
        let bpmn_index: HashMap<String, usize> = self
            .data
            .iter()
            .enumerate()
            .map(|(index, bpmn)| (bpmn.id().into(), index))
            .collect();

        self.data.iter_mut().for_each(|bpmn| match bpmn {
            Bpmn::Activity(Activity { outputs, .. }) => outputs.update_local_ids(&bpmn_index),
            Bpmn::Event(event) => {
                event.outputs.update_local_ids(&bpmn_index);
                if let Some(attached_to_ref) = &mut event.attached_to_ref {
                    attached_to_ref.update_local_id(&bpmn_index);
                }

                self.events.register(event);
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

impl TryFrom<ProcessConstruction> for ProcessData {
    type Error = ParseErrorKind;

    fn try_from(
        ProcessConstruction {
            start,
            data,
            events,
        }: ProcessConstruction,
    ) -> Result<Self, Self::Error> {
        Ok(ProcessData {
            start: start.ok_or(ParseErrorKind::MissingStartEvent)?,
            data: data.into_boxed_slice(),
            events,
        })
    }
}
