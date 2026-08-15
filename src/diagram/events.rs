use crate::{
    bpmn::{Event, EventType, Symbol},
    diagram::Id,
    process::{DiagramError, RuntimeError},
};
use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
};

#[derive(Debug, PartialEq, Eq, Hash)]
struct BoundaryKey<'a> {
    id: usize,
    symbol: Symbol,
    name: Option<Cow<'a, str>>,
}

// Need a wrapper. Ensuring the Borrow exists for different lifetimes.
// See: https://github.com/rust-lang/rust/issues/124614
#[derive(Debug, PartialEq, Eq, Hash)]
struct BoundaryKeyWrap<'a>(BoundaryKey<'a>);

impl<'short, 'long: 'short> Borrow<BoundaryKey<'short>> for BoundaryKeyWrap<'long> {
    fn borrow(&self) -> &BoundaryKey<'short> {
        &self.0
    }
}

#[derive(Default, Debug)]
pub struct Events {
    boundaries: HashMap<BoundaryKeyWrap<'static>, usize>,
    catch_event_links: HashMap<String, usize>,
}

impl Events {
    pub(super) fn register(&mut self, event: &Event) {
        match event {
            Event {
                id,
                attached_to_ref: Some(attached_to_ref),
                symbol: Some(symbol),
                name,
                ..
            } => {
                self.boundaries.insert(
                    BoundaryKeyWrap(BoundaryKey {
                        id: *attached_to_ref.local(),
                        symbol: *symbol,
                        name: name.clone().map(Cow::Owned),
                    }),
                    *id.local(),
                );
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

    pub(crate) fn boundary(&self, id: &Id, symbol: Symbol, name: Option<&str>) -> Option<&usize> {
        self.boundaries.get(&BoundaryKey {
            id: *id.local(),
            symbol,
            name: name.map(Cow::Borrowed),
        })
    }

    // Fetch all boundaries in the process. Used by scaffold only.
    pub(crate) fn boundaries(&self) -> HashMap<usize, Vec<(Option<String>, Symbol)>> {
        let mut map: HashMap<usize, Vec<_>> = HashMap::new();
        for key in self.boundaries.keys() {
            let BoundaryKey { id, symbol, name } = &key.0;

            map.entry(*id)
                .or_default()
                .push((name.clone().map(Into::into), *symbol));
        }
        map
    }

    pub(crate) fn catch_event_link(&self, throw_event_name: &str) -> Result<&usize, RuntimeError> {
        self.catch_event_links.get(throw_event_name).ok_or_else(|| {
            DiagramError::MissingIntermediateCatchEvent(
                Symbol::Link.to_string(),
                throw_event_name.into(),
            )
            .into()
        })
    }
}
