use std::collections::{HashMap, HashSet};

use crate::model::{ActivityType, Bpmn, BpmnLocal, Gateway, GatewayType};

#[derive(Debug)]
pub(super) struct Diagram {
    pub(super) data: HashMap<String, Vec<Bpmn>>,
    pub(super) definitions_id: String,
    pub(super) boundaries: HashMap<String, Vec<usize>>,
    pub(super) catch_event_links: HashMap<String, HashMap<String, usize>>,
}

impl Diagram {
    pub(super) fn install_task_function(&mut self, match_value: &str, index: usize) {
        for bpmn in self.data.values_mut().flatten() {
            if let Bpmn::Activity {
                id, func_idx, name, ..
            } = bpmn
            {
                if Diagram::match_name_or_id(name.as_deref(), id, match_value) {
                    *func_idx = Some(index)
                }
            }
        }
    }

    pub(super) fn install_gateway_function(
        &mut self,
        gw_type: GatewayType,
        match_value: &str,
        index: usize,
    ) {
        for bpmn in self.data.values_mut().flatten() {
            if let Bpmn::Gateway(Gateway {
                gateway,
                id: BpmnLocal(id, _),
                func_idx,
                name,
                ..
            }) = bpmn
            {
                if Diagram::match_name_or_id(name.as_deref(), id, match_value)
                    && gw_type == *gateway
                {
                    *func_idx = Some(index)
                }
            }
        }
    }

    pub(super) fn find_missing_functions(&self) -> HashSet<String> {
        self.data
            .values()
            .flatten()
            .fold(HashSet::new(), |mut acc, bpmn| {
                acc.insert(match bpmn {
                    Bpmn::Activity {
                        id,
                        name,
                        func_idx: None,
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
                    } => format!("{}: {}", activity, name.as_ref().unwrap_or(id)),
                    Bpmn::Gateway(Gateway {
                        gateway:
                            gateway @ (GatewayType::EventBased
                            | GatewayType::Exclusive
                            | GatewayType::Inclusive),
                        name,
                        id: BpmnLocal(id, _),
                        func_idx: None,
                        outputs,
                        ..
                    }) if outputs.len() > 1 => {
                        format!("{}: {}", gateway, name.as_ref().unwrap_or(id))
                    }
                    _ => {
                        return acc;
                    }
                });
                acc
            })
    }

    fn match_name_or_id(name: Option<&str>, id: &str, value: &str) -> bool {
        name.map(|name| name == value).unwrap_or(false) || id == value
    }
}
