use super::handler::HandlerMap;
use crate::{
    Error, Symbol,
    model::{ActivityType, Bpmn, Event, Gateway, GatewayType, Id},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub(super) struct Diagram {
    data: Vec<Vec<Bpmn>>,
    boundaries: HashMap<String, Vec<usize>>,
    catch_event_links: HashMap<String, HashMap<String, usize>>,
}

impl Diagram {
    pub(super) fn new(
        data: Vec<Vec<Bpmn>>,
        boundaries: HashMap<String, Vec<usize>>,
        catch_event_links: HashMap<String, HashMap<String, usize>>,
    ) -> Self {
        Self {
            data,
            boundaries,
            catch_event_links,
        }
    }
    // All top level processes defined in Definitions.
    // Always last in the Vec as it is a top level construct in the XML.
    pub(super) fn get_processes(&self) -> Option<&Vec<Bpmn>> {
        self.data.last()
    }

    // Can be a process or sub process
    pub(super) fn get_process(&self, process_id: usize) -> Option<&Vec<Bpmn>> {
        self.data.get(process_id)
    }

    pub(super) fn data(&self) -> &[Vec<Bpmn>] {
        self.data.as_slice()
    }

    pub(super) fn activity_boundaries(&self, id: &Id) -> Option<&Vec<usize>> {
        self.boundaries.get(id.bpmn())
    }

    pub(super) fn install_and_check(&mut self, handler_map: HandlerMap) -> HashSet<String> {
        let mut missing = HashSet::new();
        for bpmn in self.data.iter_mut().flatten() {
            match bpmn {
                Bpmn::Activity {
                    id,
                    name,
                    func_idx,
                    activity_type:
                        activity_type @ (ActivityType::Task
                        | ActivityType::ScriptTask
                        | ActivityType::UserTask
                        | ActivityType::ServiceTask
                        | ActivityType::CallActivity
                        | ActivityType::ReceiveTask
                        | ActivityType::SendTask
                        | ActivityType::ManualTask
                        | ActivityType::BusinessRuleTask),
                    ..
                } => {
                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    if let Some(id) = handler_map.task().get(name_or_id) {
                        func_idx.replace(*id);
                    } else {
                        missing.insert(format!("{activity_type}: {name_or_id}"));
                    }
                }
                Bpmn::Gateway(Gateway {
                    gateway_type:
                        gateway_type @ (GatewayType::EventBased
                        | GatewayType::Exclusive
                        | GatewayType::Inclusive),
                    name,
                    id,
                    func_idx,
                    outputs,
                    ..
                }) if outputs.len() > 1 => {
                    let map = match gateway_type {
                        GatewayType::Exclusive => handler_map.exclusive(),
                        GatewayType::Inclusive => handler_map.inclusive(),
                        GatewayType::EventBased => handler_map.event_based(),
                        _ => continue,
                    };

                    let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                    if let Some(id) = map.get(name_or_id) {
                        func_idx.replace(*id);
                    } else {
                        missing.insert(format!("{gateway_type}: {name_or_id}"));
                    }
                }
                _ => {}
            }
        }
        missing
    }

    pub(super) fn find_boundary<'a>(
        &'a self,
        activity_id: &Id,
        search_name: Option<&str>,
        search_symbol: &Symbol,
        process_data: &'a [Bpmn],
    ) -> Option<&'a usize> {
        self.activity_boundaries(activity_id)?
            .iter()
            .filter_map(|index| process_data.get(*index))
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

    pub(super) fn find_catch_link(
        &self,
        throw_event_name: &str,
        symbol: &Symbol,
        process_id: &Id,
    ) -> Result<&usize, Error> {
        self.catch_event_links
            .get(process_id.bpmn())
            .and_then(|links| links.get(throw_event_name))
            .ok_or_else(|| {
                Error::MissingIntermediateCatchEvent(symbol.to_string(), throw_event_name.into())
            })
    }
}
