use std::collections::HashMap;

use crate::{
    error::Error,
    model::{Event, Gateway, *},
    process::Diagram,
};

const BUILD_PROCESS_ERROR_MSG: &str = "couldn't build process";

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
    data: Vec<Vec<Bpmn>>,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
    process_stack: Vec<Vec<Bpmn>>,
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

    pub(super) fn add_to_process(&mut self, mut bpmn: Bpmn) {
        if let Some(current) = self.process_stack.last_mut() {
            bpmn.set_local_id(current.len());
            current.push(bpmn);
        }
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
            self.add_to_process(bpmn);
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), Error> {
        let Some((mut bpmn, mut data)) = self.stack.pop().zip(self.process_stack.pop()) else {
            return Err(Error::Builder(BUILD_PROCESS_ERROR_MSG.into()));
        };

        // Definitions only contain processes. Skip it.
        if !self.process_stack.is_empty() {
            find_and_swap_startevent(&mut data)?;
        }

        // Process or sub process use local id to point to data index. Do NOT overwrite.
        bpmn.set_local_id(self.data.len());
        self.update_data(bpmn.id()?, &mut data);
        // Definitions collect all Processes
        // Processes collect all related sub processes
        if let Some(parent_data) = self.process_stack.last_mut() {
            parent_data.push(bpmn)
        }
        self.data.push(data);
        Ok(())
    }

    // Post process to fill local ids to some bpmn objects.
    fn update_data(&mut self, process_id: &str, data: &mut [Bpmn]) {
        // Collect Bpmn id to index in array
        let bpmn_index: HashMap<String, usize> = data
            .iter()
            .enumerate()
            .filter_map(|(index, bpmn)| bpmn.id().ok().map(|id| (id.to_string(), index)))
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
                    update_local_id(attached_to_ref, &bpmn_index);

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
                        .entry(process_id.to_string())
                        .or_default()
                        .insert(name.clone(), *id.local());
                }
            }
            Bpmn::Gateway(Gateway {
                default, outputs, ..
            }) => {
                outputs.update_local_ids(&bpmn_index);
                if let Some(default) = default {
                    update_local_id(default, &bpmn_index);
                }
            }
            Bpmn::SequenceFlow { target_ref, .. } => update_local_id(target_ref, &bpmn_index),
            _ => {}
        });
    }
}

impl From<DataBuilder> for Diagram {
    fn from(builder: DataBuilder) -> Self {
        Diagram::new(builder.data, builder.boundaries, builder.catch_event_links)
    }
}

fn update_local_id(Id { bpmn_id, local_id }: &mut Id, map: &HashMap<String, usize>) {
    if let Some(index) = map.get(bpmn_id) {
        *local_id = *index;
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

fn find_and_swap_startevent(data: &mut [Bpmn]) -> Result<(), Error> {
    let index = data
        .iter()
        .position(|bpmn| {
            matches!(
                bpmn,
                Bpmn::Event(Event {
                    event_type: EventType::Start,
                    symbol: None,
                    ..
                })
            )
        })
        .ok_or(Error::MissingStartEvent)?;

    if index > 0 {
        data.swap(0, index);
        data[0].set_local_id(0);

        // Sub process use local id to point to data. Do NOT overwrite.
        let bpmn = &mut data[index];
        if !matches!(
            bpmn,
            Bpmn::Activity {
                activity_type: ActivityType::SubProcess,
                ..
            }
        ) {
            bpmn.set_local_id(index);
        }
    }
    Ok(())
}
