use crate::{bpmn::BpmnType, diagram::Id};
use std::{borrow::Cow, collections::HashMap};

#[derive(Default, Debug)]
pub struct FuncMap {
    // Use `Cow` to avoid creating an owned `String` when comparing.
    map: HashMap<(BpmnType, Cow<'static, str>), usize>,
}

impl FuncMap {
    // Check if bpmn id or name is registered by user. Begin with bpmn id as it is unique and
    // then try with the name if it exist.
    pub fn get_id(&self, ty: BpmnType, id: &Id, name: Option<&str>) -> Option<usize> {
        [Some(id.bpmn()), name]
            .into_iter()
            .flatten()
            .find_map(|s| self.map.get(&(ty, Cow::Borrowed(s))))
            .copied()
    }

    pub fn insert(&mut self, ty: BpmnType, name: String, index: usize) {
        if self
            .map
            .insert((ty, Cow::Owned(name.clone())), index)
            .is_some()
        {
            log::warn!(r#"Installed {ty} with name "{name}" multiple times"#);
        }
    }
}
