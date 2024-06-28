use std::{collections::HashMap, path::Path};

use log::error;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::error::Error;
use crate::model::*;

type ReaderResult = Result<(String, HashMap<String, HashMap<String, Bpmn>>), Error>;

// Read BPMN file and return the ReaderResult containing a tuple with definitions ID and BPMN data.
pub(crate) fn read_bpmn_file<P: AsRef<Path>>(path: P) -> ReaderResult {
    let mut reader = Reader::from_file(path)?;
    let mut data: HashMap<String, HashMap<String, Bpmn>> = HashMap::new();
    let mut process_stack: Vec<HashMap<String, Bpmn>> = Vec::new();
    let mut stack: Vec<Bpmn> = Vec::new();
    let mut buf = Vec::new();
    let mut definitions_id = None;
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
                    stack.push(Bpmn::try_from((bpmn_type, collect_attributes(&bs)))?)
                }
                bpmn_type @ (DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION) => {
                    process_stack.push(Default::default());
                    stack.push(Bpmn::try_from((bpmn_type, collect_attributes(&bs)))?)
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
                        if let Some(Bpmn::Event { symbol, .. }) = stack.last_mut() {
                            *symbol = bpmn_type.try_into().ok();
                        }
                    }
                    bpmn_type @ SEQUENCE_FLOW => {
                        let bpmn = Bpmn::try_from((bpmn_type, collect_attributes(&bs)))?;
                        if let Some((process, id)) = process_stack.last_mut().zip(bpmn.id()) {
                            process.insert(id.into(), bpmn);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(be)) => {
                match be.local_name().as_ref() {
                    OUTGOING => {
                        if let Some((
                            Bpmn::Direction {
                                text: Some(output), ..
                            },
                            parent,
                        )) = stack.pop().zip(stack.last_mut())
                        {
                            parent.set_output(output);
                        }
                    }
                    INCOMING => {
                        if let Some((
                            Bpmn::Direction {
                                text: Some(input), ..
                            },
                            parent,
                        )) = stack.pop().zip(stack.last_mut())
                        {
                            parent.set_input(input);
                        }
                    }
                    START_EVENT => {
                        if let Some((bpmn, parent)) = stack.pop().zip(stack.last_mut()) {
                            match parent {
                                Bpmn::Process { start_id, .. } => *start_id = bpmn.id().cloned(),
                                Bpmn::Activity {
                                    aktivity: ActivityType::SubProcess,
                                    start_id,
                                    ..
                                } => *start_id = bpmn.id().cloned(),
                                _ => {}
                            }
                            if let Some((process, id)) = process_stack.last_mut().zip(bpmn.id()) {
                                process.insert(id.into(), bpmn);
                            }
                        };
                    }
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
                    | SEQUENCE_FLOW => {
                        if let Some(bpmn) = stack.pop() {
                            check_unsupported(&bpmn)?;
                            if let Some((process, id)) = process_stack.last_mut().zip(bpmn.id()) {
                                process.insert(id.into(), bpmn);
                            }
                        }
                    }
                    DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION => {
                        if let Some(bpmn) = stack.pop() {
                            if let Some((process, id)) = process_stack.pop().zip(bpmn.id().cloned())
                            {
                                // Put the Bpmn model in parent scope and in 'data' it's related process data.
                                // Definitions collect all Processes
                                // Process collect all related sub processes
                                if let Some(parent_process) = process_stack.last_mut() {
                                    parent_process.insert(id.to_string(), bpmn);
                                } else {
                                    // No parent, must be Definitions.
                                    definitions_id = bpmn.id().cloned();
                                }
                                data.insert(id, process);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(bt)) => {
                if let Some(Bpmn::Direction { text, .. }) = stack.last_mut() {
                    *text = Some(bt.unescape()?.into_owned());
                }
            }

            // Ignore other XML events
            _ => (),
        }
        buf.clear();
    }
    Ok((definitions_id.ok_or(Error::MissingDefinitions)?, data))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_file() -> Result<(), Box<dyn std::error::Error>> {
        println!("{:#?}", read_bpmn_file("examples/example.bpmn")?);
        Ok(())
    }
}
