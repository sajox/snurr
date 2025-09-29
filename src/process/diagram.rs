use super::handler::HandlerMap;
use crate::{
    model::{ActivityType, Bpmn, Gateway, GatewayType},
    process::{handler::HandlerType, reader::ProcessData},
};
use std::collections::HashSet;

#[derive(Debug)]
pub(super) struct Diagram {
    data: Vec<ProcessData>,
}

impl Diagram {
    pub(super) fn new(data: Vec<ProcessData>) -> Self {
        Self { data }
    }
    // All top level processes defined in Definitions.
    // Always last in the Vec as it is a top level construct in the XML.
    pub(super) fn get_definition(&self) -> Option<&ProcessData> {
        self.data.last()
    }

    // Can be a process or sub process
    pub(super) fn get_process(&self, process_id: usize) -> Option<&ProcessData> {
        self.data.get(process_id)
    }

    pub(super) fn data(&self) -> &[ProcessData] {
        self.data.as_slice()
    }

    pub(super) fn install_and_check(&mut self, handler_map: HandlerMap) -> HashSet<String> {
        let mut missing = HashSet::new();
        for outer in self.data.iter_mut() {
            for bpmn in outer.data_mut() {
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
                        if let Some(id) = handler_map.get(HandlerType::Task, name_or_id) {
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
                        let handler_type = match gateway_type {
                            GatewayType::Exclusive => HandlerType::Exclusive,
                            GatewayType::Inclusive => HandlerType::Inclusive,
                            GatewayType::EventBased => HandlerType::EventBased,
                            _ => continue,
                        };

                        let name_or_id = name.as_deref().unwrap_or(id.bpmn());
                        if let Some(id) = handler_map.get(handler_type, name_or_id) {
                            func_idx.replace(*id);
                        } else {
                            missing.insert(format!("{gateway_type}: {name_or_id}"));
                        }
                    }
                    _ => {}
                }
            }
        }
        missing
    }
}
