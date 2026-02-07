use crate::{
    bpmn::{Event, *},
    diagram::{Diagram, ProcessData},
    error::{BUILD_PROCESS_ERROR_MSG, Error},
};

//
// data: [
//            [ // Might contain a sub process that has its data at index 1
//                Process 0 DATA
//            ],
//            [
//                Sub Process DATA
//            ],
//            [
//                Process 1 DATA
//            ],
//            [ // Definitions contains all top level processes. Always last.
//                Process 0, data at index 0
//                Process 1, data at index 2
//            ],
//        ]
//

#[derive(Default)]
pub(super) struct DataBuilder {
    data: Vec<ProcessData>,
    process_stack: Vec<ProcessData>,
    stack: Vec<Bpmn>,
}

impl DataBuilder {
    pub(super) fn add(&mut self, bpmn: Bpmn) {
        self.stack.push(bpmn);
    }

    pub(super) fn add_new_process(&mut self, bpmn: Bpmn) {
        self.process_stack.push(Default::default());
        self.add(bpmn);
    }

    pub(super) fn add_to_process(&mut self, bpmn: Bpmn) -> Result<(), Error> {
        if let Some(process_data) = self.process_stack.last_mut() {
            process_data.add(bpmn)?;
        }
        Ok(())
    }

    pub(super) fn update_symbol(&mut self, bpmn_type: &[u8]) {
        if let Some(Bpmn::Event(Event { symbol, .. })) = self.stack.last_mut() {
            *symbol = bpmn_type.try_into().ok();
        }
    }

    pub(super) fn add_direction(&mut self, direction: &[u8]) {
        if let Some(Bpmn::Direction(Some(value))) = self.stack.pop()
            && let Some(parent) = self.stack.last_mut()
        {
            match direction {
                OUTGOING => parent.add_output(value),
                _ => parent.add_input(),
            }
        }
    }

    pub(super) fn add_text(&mut self, value: String) {
        if let Some(Bpmn::Direction(text)) = self.stack.last_mut() {
            text.replace(value);
        }
    }

    pub(super) fn end(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            check_unsupported(&bpmn)?;
            self.add_to_process(bpmn)?;
        }
        Ok(())
    }

    pub(super) fn end_process(&mut self) -> Result<(), Error> {
        let Some((mut bpmn, mut process_data)) = self.stack.pop().zip(self.process_stack.pop())
        else {
            return Err(Error::Builder(BUILD_PROCESS_ERROR_MSG.into()));
        };

        // Definitions collect all Processes
        // Processes collect all related sub processes
        if let Some(parent_process_data) = self.process_stack.last_mut() {
            // Process or sub process use index to point to data.
            bpmn.update_data_index(self.data.len());
            parent_process_data.add(bpmn)?;
        }

        process_data.finalize();
        self.data.push(process_data);
        Ok(())
    }
}

impl From<DataBuilder> for Diagram {
    fn from(builder: DataBuilder) -> Self {
        Diagram::new(builder.data)
    }
}

fn check_unsupported(bpmn: &Bpmn) -> Result<(), Error> {
    Err(match bpmn {
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        Bpmn::SequenceFlow { id, name, .. } => Error::NotSupported(format!(
            "{}: {}",
            name.as_deref().unwrap_or(id.bpmn()),
            "conditional sequence flow",
        )),
        _ => return Ok(()),
    })
}
