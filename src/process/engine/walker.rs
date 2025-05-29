use std::collections::HashSet;

use crate::{
    Error,
    model::{Bpmn, Gateway, GatewayType},
};

pub fn check_gateways<'a>(
    mut gateways: Vec<&'a Gateway>,
    data: &'a [Bpmn],
) -> Result<Vec<&'a Gateway>, Error> {
    let mut related = vec![];
    let mut not_related = vec![];

    while let Some(gateway) = gateways.pop() {
        let seen = walk(*gateway.id.local(), data)?;
        if gateways
            .iter()
            .map(|v| *v.id.local())
            .any(|value| seen.contains(&value))
        {
            related.push(gateway);
        } else {
            not_related.push(gateway);
        }
    }
    Ok(if related.is_empty() {
        not_related
    } else {
        related
    })
}

pub fn walk(current_id: usize, data: &[Bpmn]) -> Result<HashSet<usize>, Error> {
    let mut walker = Walker::default();
    walker.add(current_id);
    while let Some(value) = walker.pop() {
        match data
            .get(value)
            .ok_or_else(|| Error::MisssingBpmnData(current_id.to_string()))?
        {
            Bpmn::Event { outputs, .. } | Bpmn::Activity { outputs, .. } => {
                walker.add_slice(outputs.ids());
            }
            Bpmn::Gateway(Gateway {
                gateway: GatewayType::Inclusive,
                outputs,
                ..
            }) => {
                walker.add_slice(outputs.ids());
            }
            Bpmn::SequenceFlow { target_ref, .. } => {
                walker.add(*target_ref.local());
            }
            _ => {}
        };
    }
    Ok(walker.set())
}

#[derive(Debug, Default)]
struct Walker {
    visited: HashSet<usize>,
    list: Vec<usize>,
}

impl Walker {
    fn add(&mut self, value: usize) {
        if self.visited.insert(value) {
            self.list.push(value);
        }
    }

    fn add_slice(&mut self, value: &[usize]) {
        value.iter().for_each(|value| self.add(*value));
    }

    fn pop(&mut self) -> Option<usize> {
        self.list.pop()
    }

    fn set(self) -> HashSet<usize> {
        self.visited
    }
}
