use crate::{
    Error, Symbol,
    bpmn::{Event, EventType},
    diagram::Id,
};
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct Events {
    boundaries: HashMap<usize, Vec<usize>>,
    catch_event_links: HashMap<String, usize>,
}

impl Events {
    pub(super) fn register(&mut self, event: &Event) {
        match event {
            Event {
                id,
                attached_to_ref: Some(attached_to_ref),
                ..
            } => {
                self.boundaries
                    .entry(*attached_to_ref.local())
                    .or_default()
                    .push(*id.local());
            }
            Event {
                event_type: EventType::IntermediateCatch,
                symbol: Some(Symbol::Link),
                id,
                name: Some(name),
                ..
            } => {
                self.catch_event_links.insert(name.clone(), *id.local());
            }
            _ => {}
        }
    }

    pub(crate) fn boundaries(&self, id: &Id) -> Option<&Vec<usize>> {
        self.boundaries.get(id.local())
    }

    pub(crate) fn catch_event_link(&self, throw_event_name: &str) -> Result<&usize, Error> {
        self.catch_event_links.get(throw_event_name).ok_or_else(|| {
            Error::MissingIntermediateCatchEvent(Symbol::Link.to_string(), throw_event_name.into())
        })
    }
}
