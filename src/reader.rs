use std::{collections::HashMap, path::Path};

use log::error;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::error::Error;
use crate::model::{Bpmn, BpmnAttrib, BpmnType};

type ReaderResult = Result<(HashMap<String, Bpmn>, HashMap<String, String>), Error>;

// Read BPMN file and return ReaderResult containing a tuple with BPMN data and start ids.
pub(crate) fn read_bpmn_file<P: AsRef<Path>>(path: P) -> ReaderResult {
    let mut reader = Reader::from_file(path).map_err(Error::File)?;
    reader.trim_text(true);

    let mut data: HashMap<String, Bpmn> = HashMap::new();
    let mut stack: Vec<Bpmn> = Vec::new();
    let mut start_ids: HashMap<String, String> = HashMap::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => error!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(bs)) => {
                let bpmn_type: BpmnType = bs.local_name().as_ref().into();
                match bpmn_type {
                    BpmnType::StartEvent
                    | BpmnType::EndEvent
                    | BpmnType::BoundaryEvent
                    | BpmnType::IntermediateCatchEvent
                    | BpmnType::IntermediateThrowEvent
                    | BpmnType::Task
                    | BpmnType::Outgoing
                    | BpmnType::Incoming
                    | BpmnType::ExclusiveGateway
                    | BpmnType::Process
                    | BpmnType::SubProcess
                    | BpmnType::ParallelGateway
                    | BpmnType::InclusiveGateway => {
                        stack.push(Bpmn::try_from((bpmn_type, collect_attributes(bs)))?)
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(bs)) => {
                let bpmn_type: BpmnType = bs.local_name().as_ref().into();
                match bpmn_type {
                    // Attach symbol to parent
                    BpmnType::ErrorEventDefinition
                    | BpmnType::MessageEventDefinition
                    | BpmnType::TimerEventDefinition
                    | BpmnType::EscalationEventDefinition
                    | BpmnType::ConditionalEventDefinition
                    | BpmnType::SignalEventDefinition
                    | BpmnType::CompensateEventDefinition => {
                        if let Some(Bpmn::Event {
                            event: _,
                            symbol,
                            id: _,
                            name: _,
                            attached_to_ref: _,
                            cancel_activity: _,
                            output: _,
                        }) = stack.last_mut()
                        {
                            *symbol = bpmn_type.try_into().ok();
                        }
                    }
                    BpmnType::SequenceFlow => {
                        let bpmn = Bpmn::try_from((bpmn_type, collect_attributes(bs)))?;
                        if let Some(id) = bpmn.id() {
                            data.insert(id.into(), bpmn);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(be)) => {
                let bpmn_type: BpmnType = be.local_name().as_ref().into();
                match bpmn_type {
                    BpmnType::Outgoing => {
                        if let Some((
                            Bpmn::Direction {
                                direction: BpmnType::Outgoing,
                                text: Some(out),
                            },
                            parent,
                        )) = stack.pop().zip(stack.last_mut())
                        {
                            parent.set_output(out);
                        }
                    }
                    BpmnType::Incoming => {
                        // Silently remove incoming as we dont handle it. Easy to add if support needed.
                        stack.pop();
                    }
                    BpmnType::StartEvent
                    | BpmnType::EndEvent
                    | BpmnType::BoundaryEvent
                    | BpmnType::IntermediateCatchEvent
                    | BpmnType::IntermediateThrowEvent
                    | BpmnType::Task
                    | BpmnType::ExclusiveGateway
                    | BpmnType::Process
                    | BpmnType::SubProcess
                    | BpmnType::ParallelGateway
                    | BpmnType::InclusiveGateway => {
                        if let Some(bpmn) = stack.pop() {
                            let Some(id) = bpmn.id() else {
                                continue;
                            };

                            if bpmn_type == BpmnType::StartEvent {
                                if let Some(parent_id) = stack.last().and_then(|bpmn| bpmn.id()) {
                                    start_ids.insert(parent_id.into(), id.into());
                                }
                            }
                            data.insert(id.into(), bpmn);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(bt)) => {
                if let Some(Bpmn::Direction { direction: _, text }) = stack.last_mut() {
                    *text = Some(bt.unescape()?.into_owned());
                }
            }

            // Ignore other XML events
            _ => (),
        }
        buf.clear();
    }

    Ok((data, start_ids))
}

fn collect_attributes(bs: quick_xml::events::BytesStart<'_>) -> HashMap<BpmnAttrib, String> {
    bs.attributes()
        .filter_map(Result::ok)
        .map(|attribute| {
            (
                attribute.key.local_name().into_inner().into(),
                std::str::from_utf8(attribute.value.as_ref())
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .filter(|(_, s)| !s.is_empty())
        .collect::<HashMap<BpmnAttrib, String>>()
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
