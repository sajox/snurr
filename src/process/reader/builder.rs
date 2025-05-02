use crate::{
    Process,
    error::Error,
    model::{Gateway, *},
};

const BUILD_PROCESS_ERROR_MSG: &str = "couldn't build process";

#[derive(Default)]
pub(super) struct DataBuilder {
    data: HashMap<String, Vec<Bpmn>>,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
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
        let Some((bpmn, mut data)) = self.stack.pop().zip(self.process_stack.pop()) else {
            return Err(Error::Builder(BUILD_PROCESS_ERROR_MSG.into()));
        };

        let id = bpmn.id()?.to_string();
        self.update_data(&id, &mut data);
        // Definitions collect all Processes
        // Processes collect all related sub processes
        match self.process_stack.last_mut() {
            Some(parent_data) => parent_data.push(bpmn),
            None => self.definitions_id = Some(id.clone()),
        }
        self.data.insert(id, data);
        Ok(())
    }

    fn update_data(&mut self, process_id: &str, data: &mut [Bpmn]) {
        // Collect Bpmn id to index in array
        let bpmn_index: HashMap<String, usize> = data
            .iter()
            .enumerate()
            .filter_map(|(index, bpmn)| bpmn.id().ok().map(|id| (id.to_string(), index)))
            .collect();

        data.iter_mut().for_each(|bpmn| match bpmn {
            Bpmn::Activity { outputs, .. } => outputs.update_local_ids(&bpmn_index),
            Bpmn::Event {
                event,
                id,
                outputs,
                attached_to_ref,
                symbol,
                name,
                ..
            } => {
                outputs.update_local_ids(&bpmn_index);
                update_local_id(id, &bpmn_index);
                if let Some(attached_to_ref) = attached_to_ref {
                    update_local_id(attached_to_ref, &bpmn_index);

                    // Collect boundary to activity id
                    self.boundaries
                        .entry(attached_to_ref.bpmn().to_owned())
                        .or_default()
                        .push(*id.local());
                }

                if *event == EventType::IntermediateCatch
                    && *symbol == Some(Symbol::Link)
                    && name.is_some()
                {
                    self.catch_event_links
                        .entry(process_id.to_string())
                        .or_default()
                        .insert(name.clone().unwrap(), *id.local());
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

impl TryFrom<DataBuilder> for Process {
    type Error = Error;

    fn try_from(builder: DataBuilder) -> Result<Self, Self::Error> {
        Ok(Self {
            data: builder.data,
            definitions_id: builder.definitions_id.ok_or(Error::MissingDefinitionsId)?,
            boundaries: builder.boundaries,
            catch_event_links: builder.catch_event_links,
        })
    }
}

fn update_local_id(BpmnLocal(bid, lid): &mut BpmnLocal, map: &HashMap<String, usize>) {
    if let Some(index) = map.get(bid) {
        *lid = *index;
    }
}

fn check_unsupported(bpmn: &Bpmn) -> Result<(), Error> {
    Err(match bpmn {
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        Bpmn::SequenceFlow { id, name, .. } => Error::NotSupported(format!(
            "{}: {}",
            name.as_ref().unwrap_or(id),
            "conditional sequence flow",
        )),
        _ => return Ok(()),
    })
}
