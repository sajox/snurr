use std::{collections::HashMap, path::Path};

use log::error;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::error::Error;
use crate::model::*;

type ReaderResult = Result<(String, HashMap<String, HashMap<String, Bpmn>>), Error>;

// Read BPMN file and return the ReaderResult containing a tuple with definitions ID and BPMN data.
pub(crate) fn read_bpmn_file<P: AsRef<Path>>(path: P) -> ReaderResult {
    let mut builder = DataBuilder::default();
    let mut reader = Reader::from_file(path)?;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => error!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(bs)) => match bs.local_name().as_ref() {
                bpmn_type @ (START_EVENT
                | END_EVENT
                | BOUNDARY_EVENT
                | INTERMEDIATE_CATCH_EVENT
                | INTERMEDIATE_THROW_EVENT
                | TASK
                | SCRIPT_TASK
                | USER_TASK
                | SERVICE_TASK
                | CALL_ACTIVITY
                | RECEIVE_TASK
                | SEND_TASK
                | MANUAL_TASK
                | BUSINESS_RULE_TASK
                | OUTGOING
                | INCOMING
                | EXCLUSIVE_GATEWAY
                | PARALLEL_GATEWAY
                | INCLUSIVE_GATEWAY
                | SEQUENCE_FLOW) => {
                    builder.add(Bpmn::try_from((bpmn_type, collect_attributes(&bs)))?)
                }
                bpmn_type @ (DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION) => {
                    builder.add_new_process(Bpmn::try_from((bpmn_type, collect_attributes(&bs)))?)
                }
                _ => {}
            },
            Ok(Event::Empty(bs)) => {
                match bs.local_name().as_ref() {
                    // Attach symbol to parent
                    bpmn_type @ (CANCEL_EVENT_DEFINITION
                    | COMPENSATE_EVENT_DEFINITION
                    | CONDITIONAL_EVENT_DEFINITION
                    | ERROR_EVENT_DEFINITION
                    | ESCALATION_EVENT_DEFINITION
                    | MESSAGE_EVENT_DEFINITION
                    | LINK_EVENT_DEFINITION
                    | SIGNAL_EVENT_DEFINITION
                    | TERMINATE_EVENT_DEFINITION
                    | TIMER_EVENT_DEFINITION) => {
                        builder.update_symbol(bpmn_type);
                    }
                    bpmn_type @ SEQUENCE_FLOW => {
                        builder.add_to_process(Bpmn::try_from((
                            bpmn_type,
                            collect_attributes(&bs),
                        ))?)?;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(be)) => match be.local_name().as_ref() {
                direction @ (OUTGOING | INCOMING) => builder.add_direction(direction),
                START_EVENT => builder.update_start_id()?,
                END_EVENT
                | BOUNDARY_EVENT
                | INTERMEDIATE_CATCH_EVENT
                | INTERMEDIATE_THROW_EVENT
                | TASK
                | SCRIPT_TASK
                | USER_TASK
                | SERVICE_TASK
                | CALL_ACTIVITY
                | RECEIVE_TASK
                | SEND_TASK
                | MANUAL_TASK
                | BUSINESS_RULE_TASK
                | EXCLUSIVE_GATEWAY
                | PARALLEL_GATEWAY
                | INCLUSIVE_GATEWAY
                | SEQUENCE_FLOW => builder.end()?,
                DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION => builder.end_process()?,
                _ => {}
            },
            Ok(Event::Text(bt)) => {
                builder.add_text(bt.unescape()?.into_owned())?;
            }

            // Ignore other XML events
            _ => (),
        }
        buf.clear();
    }
    Ok((
        builder.definitions_id.ok_or(Error::MissingDefinitions)?,
        builder.data,
    ))
}

fn collect_attributes<'a>(bs: &'a quick_xml::events::BytesStart<'_>) -> HashMap<&'a [u8], String> {
    bs.attributes()
        .filter_map(Result::ok)
        .map(|attribute| {
            (
                attribute.key.local_name().into_inner(),
                String::from_utf8(attribute.value.to_vec()).unwrap_or_default(),
            )
        })
        .filter(|(_, s)| !s.is_empty())
        .collect::<HashMap<&'a [u8], String>>()
}

fn check_unsupported(bpmn: &Bpmn) -> Result<(), Error> {
    Err(match bpmn {
        Bpmn::Gateway {
            gateway: GatewayType::Parallel | GatewayType::Inclusive,
            id,
            name,
            outputs,
            inputs,
            ..
        } if outputs.len() > 1 && inputs.len() > 1 => Error::NotSupported(format!(
            "{}: {}",
            name.as_ref().unwrap_or(id),
            "Both Join and Fork",
        )),
        // SequenceFlow with Start and End tag is Conditional Sequence Flow
        Bpmn::SequenceFlow { id, name, .. } => Error::NotSupported(format!(
            "{}: {}",
            name.as_ref().unwrap_or(id),
            "Conditional Sequence Flow",
        )),
        _ => return Ok(()),
    })
}

#[derive(Default)]
struct DataBuilder {
    data: HashMap<String, HashMap<String, Bpmn>>,
    process_stack: Vec<HashMap<String, Bpmn>>,
    stack: Vec<Bpmn>,
    definitions_id: Option<String>,
}

impl DataBuilder {
    fn add(&mut self, bpmn: Bpmn) {
        self.stack.push(bpmn);
    }

    fn add_new_process(&mut self, bpmn: Bpmn) {
        self.process_stack.push(Default::default());
        self.add(bpmn);
    }

    fn add_to_process(&mut self, bpmn: Bpmn) -> Result<(), Error> {
        if let Some(process) = self.process_stack.last_mut() {
            process.insert(bpmn.id()?.into(), bpmn);
        }
        Ok(())
    }

    fn update_symbol(&mut self, bpmn_type: &[u8]) {
        if let Some(Bpmn::Event { symbol, .. }) = self.stack.last_mut() {
            *symbol = bpmn_type.try_into().ok();
        }
    }

    fn add_direction(&mut self, direction: &[u8]) {
        if let Some((
            Bpmn::Direction {
                text: Some(value), ..
            },
            parent,
        )) = self.stack.pop().zip(self.stack.last_mut())
        {
            match direction {
                OUTGOING => parent.add_output(value),
                _ => parent.add_input(value),
            }
        }
    }

    fn update_start_id(&mut self) -> Result<(), Error> {
        if let Some((
            bpmn,
            Bpmn::Process { start_id, .. }
            | Bpmn::Activity {
                aktivity: ActivityType::SubProcess,
                start_id,
                ..
            },
        )) = self.stack.pop().zip(self.stack.last_mut())
        {
            // Update parent process or subprocess start_id
            *start_id = Some(bpmn.id()?.into());
            self.add_to_process(bpmn)?
        }
        Ok(())
    }

    fn add_text(&mut self, value: String) -> Result<(), Error> {
        if let Some(Bpmn::Direction { text, .. }) = self.stack.last_mut() {
            *text = Some(value);
        }
        Ok(())
    }

    fn end(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            check_unsupported(&bpmn)?;
            self.add_to_process(bpmn)?
        }
        Ok(())
    }

    fn end_process(&mut self) -> Result<(), Error> {
        if let Some(bpmn) = self.stack.pop() {
            if let Some(process) = self.process_stack.pop() {
                let id = bpmn.id()?.to_string();
                // Put the Bpmn model in parent scope and in 'data' it's related process data.
                // Definitions collect all Processes
                // Process collect all related sub processes
                if let Some(parent_process) = self.process_stack.last_mut() {
                    parent_process.insert(id.clone(), bpmn);
                } else {
                    // No parent, must be Definitions.
                    self.definitions_id = Some(id.clone());
                }
                self.data.insert(id, process);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_file() -> Result<(), Box<dyn std::error::Error>> {
        println!("{:#?}", read_bpmn_file("examples/example.bpmn")?);
        Ok(())
    }
}
