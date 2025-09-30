use crate::{
    error::{Error, ONLY_ONE_START_EVENT},
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
    boundaries: HashMap<usize, Vec<usize>>,
    catch_event_links: HashMap<String, usize>,
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
            return Err(Error::BpmnRequirement(ONLY_ONE_START_EVENT.into()));
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
            .filter_map(|(index, bpmn)| bpmn.id().ok().map(Id::bpmn).map(|id| (id.into(), index)))
            .collect();

        self.data.iter_mut().for_each(|bpmn| match bpmn {
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
                        .entry(*attached_to_ref.local())
                        .or_default()
                        .push(*id.local());
                }

                if let Some(name) = name
                    && let Some(Symbol::Link) = symbol
                    && EventType::IntermediateCatch == *event_type
                {
                    self.catch_event_links.insert(name.clone(), *id.local());
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

    pub fn activity_boundaries(&self, id: &Id) -> Option<&Vec<usize>> {
        self.boundaries.get(id.local())
    }

    pub fn find_boundary<'a>(
        &'a self,
        activity_id: &Id,
        search_name: Option<&str>,
        search_symbol: &Symbol,
    ) -> Option<&'a usize> {
        self.activity_boundaries(activity_id)?
            .iter()
            .filter_map(|index| self.data.get(*index))
            .find_map(|bpmn| match bpmn {
                Bpmn::Event(Event {
                    symbol: Some(symbol),
                    id,
                    name,
                    ..
                }) if symbol == search_symbol && search_name == name.as_deref() => Some(id.local()),
                _ => None,
            })
    }

    pub fn catch_event_link(&self, throw_event_name: &str) -> Result<&usize, Error> {
        self.catch_event_links.get(throw_event_name).ok_or_else(|| {
            Error::MissingIntermediateCatchEvent(Symbol::Link.to_string(), throw_event_name.into())
        })
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

        // Definitions collect all Processes
        // Processes collect all related sub processes
        if let Some(parent_process_data) = self.process_stack.last_mut() {
            // Process or sub process use index to point to data.
            bpmn.update_data_index(self.data.len());
            parent_process_data.add(bpmn)?;
        }

        process_data.finalize();
        self.data.push(process_data);
        Ok(())
    }
}

impl From<DataBuilder> for Diagram {
    fn from(builder: DataBuilder) -> Self {
        Diagram::new(builder.data)
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
