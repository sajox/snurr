use core::fmt;
use std::{
    collections::HashMap,
    error::Error,
    fmt::{Display, Formatter},
};

use crate::{
    bpmn::{Event, *},
    diagram::{Diagram, Id, Outputs, ProcessData, events::Events, reader::RawData},
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
    definitions: Vec<usize>,

    // Process and subprocess data
    data: Vec<ProcessData>,
    process_stack: Vec<ProcessConstruction>,
    stack: Vec<RawData>,

    // Temporary text from XML
    text: Option<String>,
}

impl DataBuilder {
    pub(super) fn add(&mut self, raw_data: RawData) {
        self.stack.push(raw_data);
    }

    pub(super) fn add_new_process(&mut self, raw_data: RawData) {
        self.process_stack.push(Default::default());
        self.add(raw_data);
    }

    pub(super) fn add_to_process(&mut self, raw_data: RawData) -> Result<(), ParseError> {
        if let Some(process_data) = self.process_stack.last_mut() {
            process_data.add(Bpmn::try_from(raw_data).map_err(ParseErrorKind::Bpmn)?)?;
        }
        Ok(())
    }

    pub(super) fn update_symbol(&mut self, bpmn_type: &str) {
        if let Some(RawData { symbol, .. }) = self.stack.last_mut() {
            *symbol = bpmn_type.try_into().ok();
        }
    }

    pub(super) fn add_text_to_parent(&mut self, bpmn_type: &str) {
        if let Some(parent) = self.stack.last_mut()
            && let Some(text) = self.text.take()
        {
            match bpmn_type {
                OUTGOING => parent.outputs.push(text),
                _ => parent.inputs.push(text),
            }
        }
    }

    pub(super) fn add_text(&mut self, value: impl Into<String>) {
        self.text.replace(value.into());
    }

    pub(super) fn end(&mut self) -> Result<(), ParseError> {
        if let Some(xml_data) = self.stack.pop() {
            check_unsupported(&xml_data)?;
            self.add_to_process(xml_data)?;
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), ParseError> {
        let Some((mut xml_data, mut process_data)) = self.stack.pop().zip(self.process_stack.pop())
        else {
            Err(ParseErrorKind::ProcessBuild)?
        };

        match self.process_stack.last_mut() {
            // Processes collect all related subprocesses
            Some(parent_process_data) => {
                // sub process use index to point to data.
                xml_data.data_index = Some(self.data.len());
                let bpmn = Bpmn::try_from(xml_data).map_err(ParseErrorKind::Bpmn)?;
                parent_process_data.add(bpmn)?;
            }
            // Definitions collect all processes. data.len() is process index.
            None => self.definitions.push(self.data.len()),
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

fn check_unsupported(raw_data: &RawData) -> Result<(), ParseErrorKind> {
    Err(match raw_data {
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        RawData {
            bpmn_type,
            attributes,
            ..
        } if bpmn_type == SEQUENCE_FLOW => ParseErrorKind::NotSupported(format!(
            "{}: {}",
            raw_data.name_or_id().map_err(ParseErrorKind::Bpmn)?,
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

impl TryFrom<RawData> for Bpmn {
    type Error = BpmnError;

    fn try_from(
        RawData {
            bpmn_type,
            mut attributes,
            symbol,
            outputs,
            inputs,
            data_index,
        }: RawData,
    ) -> Result<Self, Self::Error> {
        let bpmn_type: &str = bpmn_type.as_ref();
        let id: Id = attributes
            .remove(&Attrib::Id)
            .ok_or_else(|| BpmnError::MissingId(bpmn_type.to_owned()))?
            .into();
        let name = attributes.remove(&Attrib::Name);

        let ty = match bpmn_type {
            START_EVENT
            | END_EVENT
            | BOUNDARY_EVENT
            | INTERMEDIATE_CATCH_EVENT
            | INTERMEDIATE_THROW_EVENT => Bpmn::Event(Event {
                event_type: bpmn_type.try_into()?,
                symbol,
                id,
                name,
                attached_to_ref: attributes.remove(&Attrib::AttachedToRef).map(Into::into),
                outputs: Outputs::new(outputs),
            }),
            TASK | SCRIPT_TASK | USER_TASK | SERVICE_TASK | CALL_ACTIVITY | RECEIVE_TASK
            | SEND_TASK | MANUAL_TASK | BUSINESS_RULE_TASK | SUB_PROCESS | TRANSACTION => {
                Bpmn::Activity(Activity {
                    activity_type: bpmn_type.try_into()?,
                    id,
                    func_idx: None,
                    data_index,
                    name,
                    outputs: Outputs::new(outputs),
                })
            }
            EXCLUSIVE_GATEWAY | PARALLEL_GATEWAY | INCLUSIVE_GATEWAY | EVENT_BASED_GATEWAY => {
                Bpmn::Gateway(Gateway {
                    gateway_type: bpmn_type.try_into()?,
                    id,
                    func_idx: None,
                    name,
                    default: attributes.remove(&Attrib::Default).map(Into::into),
                    outputs: Outputs::new(outputs),
                    inputs: inputs.len() as u16,
                })
            }
            SEQUENCE_FLOW => Bpmn::SequenceFlow {
                id,
                name,
                target_ref: attributes
                    .remove(&Attrib::TargetRef)
                    .ok_or(BpmnError::MissingTargetRef)?
                    .into(),
            },
            _ => Err(BpmnError::TypeNotImplemented(bpmn_type.into()))?,
        };

        ty.validate()?;
        Ok(ty)
    }
}

/// Errors that can occur while constructing bpmn types.
#[derive(Debug)]
pub enum BpmnError {
    MissingId(String),
    MissingTargetRef,
    NoOutput(String),
    BpmnRequirement(String),
    TypeNotImplemented(String),
}

impl Display for BpmnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BpmnError::MissingId(s) => write!(f, "tag `{s}` missing attribute id"),
            BpmnError::MissingTargetRef => {
                f.write_str("tag `sequenceFlow` missing attribute targetRef")
            }
            BpmnError::NoOutput(s) => write!(f, "{s} has no output"),
            BpmnError::BpmnRequirement(s) => write!(f, "{s}"),
            BpmnError::TypeNotImplemented(s) => write!(f, "tag `{s}` not implemented"),
        }
    }
}

impl Error for BpmnError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
