use super::handler::HandlerMap;
use crate::model::{ActivityType, Bpmn, Gateway, GatewayType};
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

    pub(super) fn boundaries(&self) -> &HashMap<String, Vec<usize>> {
        &self.boundaries
    }

    pub(super) fn catch_event_links(&self) -> &HashMap<String, HashMap<String, usize>> {
        &self.catch_event_links
    }

    pub(super) fn install_and_check(&mut self, handler_map: HandlerMap) -> HashSet<String> {
        let mut missing = HashSet::new();
        for bpmn in self.data.iter_mut().flatten() {
            match bpmn {
                Bpmn::Activity {
                    id,
                    name,
                    func_idx,
                    activity:
                        activity @ (ActivityType::Task
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
                    let name_or_id: &str = name.as_deref().unwrap_or(id.bpmn());
                    if let Some(id) = handler_map.task().get(name_or_id) {
                        func_idx.replace(*id);
                    } else {
                        missing.insert(format!("{}: {}", activity, name_or_id));
                    }
                }
                Bpmn::Gateway(Gateway {
                    gateway:
                        gateway @ (GatewayType::EventBased
                        | GatewayType::Exclusive
                        | GatewayType::Inclusive),
                    name,
                    id,
                    func_idx,
                    outputs,
                    ..
                }) if outputs.len() > 1 => {
                    let map = match gateway {
                        GatewayType::Exclusive => handler_map.exclusive(),
                        GatewayType::Inclusive => handler_map.inclusive(),
                        GatewayType::EventBased => handler_map.event_based(),
                        _ => continue,
                    };

                    let name_or_id: &str = name.as_deref().unwrap_or(id.bpmn());
                    if let Some(id) = map.get(name_or_id) {
                        func_idx.replace(*id);
                    } else {
                        missing.insert(format!("{}: {}", gateway, name_or_id));
                    }
                }
                _ => {}
            }
        }
        missing
    }
}
