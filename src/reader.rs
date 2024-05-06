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

    let mut bpmn_data: HashMap<String, Bpmn> = HashMap::new();
    let mut temp: Vec<Bpmn> = Vec::new();
    let mut start_ids: HashMap<String, String> = HashMap::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => error!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            Ok(Event::Start(bs)) | Ok(Event::Empty(bs)) => {
                let bpmn_type: BpmnType = bs.local_name().as_ref().into();
                match bpmn_type {
                    BpmnType::Ignore => {}
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
                        }) = temp.last_mut()
                        {
                            *symbol = bpmn_type.try_into().ok();
                        }
                    }
                    BpmnType::SequenceFlow => {
                        if let Ok((Some(id), diagram)) =
                            Bpmn::try_from((bpmn_type.clone(), collect_attributes(bs)))
                                .map(|diagram| (diagram.id().cloned(), diagram))
                        {
                            bpmn_data.insert(id, diagram);
                        }
                    }
                    _ => {
                        if let Ok(diagram) =
                            Bpmn::try_from((bpmn_type.clone(), collect_attributes(bs)))
                        {
                            temp.push(diagram);
                        }
                    }
                }
            }
            Ok(Event::End(be)) => {
                let bpmn_type: BpmnType = be.local_name().as_ref().into();
                match bpmn_type {
                    BpmnType::Ignore | BpmnType::Definitions | BpmnType::SequenceFlow => {}
                    BpmnType::Incoming | BpmnType::Outgoing => {
                        if let Some((
                            Bpmn::Direction {
                                direction: BpmnType::Outgoing,
                                text: Some(out),
                            },
                            parent,
                        )) = temp.pop().zip(temp.last_mut())
                        {
                            parent.set_output(out);
                        }
                    }
                    _ => {
                        if let Some((Some(id), diagram)) =
                            temp.pop().map(|diagram| (diagram.id().cloned(), diagram))
                        {
                            if bpmn_type == BpmnType::StartEvent {
                                if let Some(parent_id) =
                                    temp.last().and_then(|diagram| diagram.id())
                                {
                                    start_ids.insert(parent_id.to_string(), id.to_string());
                                }
                            }

                            bpmn_data.insert(id, diagram);
                        }
                    }
                }
            }
            Ok(Event::Text(bt)) => {
                if let Some(Bpmn::Direction { direction: _, text }) = temp.last_mut() {
                    *text = Some(bt.unescape()?.into_owned());
                }
            }
            _ => (),
        }
        buf.clear();
    }

    Ok((bpmn_data, start_ids))
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
