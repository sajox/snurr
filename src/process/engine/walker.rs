use std::collections::HashSet;

use crate::{
    Error,
    model::{Bpmn, Gateway},
};

pub fn check_gateways<'a>(
    gateways: Vec<&'a Gateway>,
    data: &'a [Bpmn],
) -> Result<Vec<&'a Gateway>, Error> {
    let mut related = vec![];
    let mut not_related = vec![];

    for &gateway in gateways.iter() {
        let seen = walk(gateway, data)?;
        if gateways
            .iter()
            .filter(|v| v.id.local() != gateway.id.local())
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

pub fn walk(gateway: &Gateway, data: &[Bpmn]) -> Result<HashSet<usize>, Error> {
    let mut walker = Walker::default();
    walker.add_slice(gateway.outputs.ids());
    while let Some(value) = walker.pop() {
        match data
            .get(value)
            .ok_or_else(|| Error::MisssingBpmnData(value.to_string()))?
        {
            Bpmn::Gateway(Gateway { outputs, .. })
            | Bpmn::Event { outputs, .. }
            | Bpmn::Activity { outputs, .. } => {
                walker.add_slice(outputs.ids());
            }
            Bpmn::SequenceFlow { target_ref, .. } => {
                walker.add(*target_ref.local());
            }
            _ => {}
        };
    }
    Ok(walker.visited())
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

    fn visited(self) -> HashSet<usize> {
        self.visited
    }
}
