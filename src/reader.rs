mod builder;

use std::io::BufRead;
use std::{collections::HashMap, path::Path};

use builder::DataBuilder;
use log::error;
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::error::Error;
use crate::model::*;

type ReaderResult = Result<(String, HashMap<String, Vec<Bpmn>>), Error>;

pub(crate) fn read_bpmn_str(s: &str) -> ReaderResult {
    read_bpmn(Reader::from_str(s))
}

pub(crate) fn read_bpmn_file<P: AsRef<Path>>(path: P) -> ReaderResult {
    read_bpmn(Reader::from_file(path)?)
}

// Read BPMN content and return the ReaderResult containing a tuple with definitions ID and BPMN data.
fn read_bpmn<R: BufRead>(mut reader: Reader<R>) -> ReaderResult {
    let mut builder = DataBuilder::default();
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
                        builder
                            .add_to_process(Bpmn::try_from((bpmn_type, collect_attributes(&bs)))?);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(be)) => match be.local_name().as_ref() {
                direction @ (OUTGOING | INCOMING) => builder.add_direction(direction),
                START_EVENT
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
                | EXCLUSIVE_GATEWAY
                | PARALLEL_GATEWAY
                | INCLUSIVE_GATEWAY
                | SEQUENCE_FLOW => builder.end()?,
                DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION => builder.end_process()?,
                _ => {}
            },
            Ok(Event::Text(bt)) => {
                builder.add_text(bt.unescape()?.into_owned());
            }

            // Ignore other XML events
            _ => (),
        }
        buf.clear();
    }
    builder.finish()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_file() -> Result<(), Box<dyn std::error::Error>> {
        println!("{:#?}", read_bpmn_file("examples/example.bpmn")?);
        Ok(())
    }
}
