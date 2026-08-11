mod builder;
pub use builder::BpmnError;
pub use builder::BpmnErrorKind;

use super::Diagram;
use crate::{
    bpmn::*,
    process::{ParseError, ParseErrorKind},
};
use builder::DataBuilder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::BufRead;

// Read BPMN content and return the Diagram
pub fn read_bpmn<R: BufRead>(mut reader: Reader<R>) -> Result<Diagram, ParseError> {
    let mut builder = DataBuilder::default();

    // We keep all content to be able to fetch line and column number if errors occur.
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Err(e) => {
                let (line, column) = reader.line_and_column(&buf)?;
                Err(ParseErrorKind::Xml {
                    line,
                    column,
                    source: e.into(),
                })?
            }
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
                | EXCLUSIVE_GATEWAY
                | PARALLEL_GATEWAY
                | INCLUSIVE_GATEWAY
                | EVENT_BASED_GATEWAY
                | SEQUENCE_FLOW) => {
                    builder.add(RawData::new(bpmn_type, collect_attributes(&bs)));
                }
                bpmn_type @ (PROCESS | SUB_PROCESS | TRANSACTION) => {
                    builder.add_new_process(RawData::new(bpmn_type, collect_attributes(&bs)));
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
                        builder.add_to_process(RawData::new(bpmn_type, collect_attributes(&bs)))?;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(be)) => match be.local_name().as_ref() {
                bpmn_type @ (OUTGOING | INCOMING) => builder.add_text_to_parent(bpmn_type),
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
                | EVENT_BASED_GATEWAY
                | SEQUENCE_FLOW => builder.end()?,
                PROCESS | SUB_PROCESS | TRANSACTION => builder.end_process()?,
                _ => {}
            },
            Ok(Event::Text(bt)) => {
                builder.add_text(bt.into_inner());
            }
            // Ignore other XML events
            _ => (),
        }
    }
    Ok(builder.into())
}

fn collect_attributes(bs: &quick_xml::events::BytesStart<'_>) -> HashMap<Attrib, String> {
    bs.attributes()
        .filter_map(Result::ok)
        .filter(|attribute| !attribute.value.is_empty())
        .filter_map(|attribute| {
            Some((
                attribute.key.local_name().into_inner().try_into().ok()?,
                attribute.value.into(),
            ))
        })
        .collect::<HashMap<Attrib, String>>()
}

trait LineAndColumn {
    fn line_and_column(&self, data: &[u8]) -> Result<(usize, usize), ParseError>;
}

impl<T> LineAndColumn for Reader<T> {
    fn line_and_column(&self, data: &[u8]) -> Result<(usize, usize), ParseError> {
        let end_pos = self.error_position() as usize;
        let content = String::from_utf8(data[0..end_pos].to_owned())
            .map_err(|e| ParseErrorKind::Encoding(e.into()))?;
        let mut line = 1;
        let mut column = 0;
        for c in content.chars() {
            if c == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
        Ok((line, column))
    }
}

// Temporary objekt to collect the complete element (start and end-tag) before trying to create the Bpmn type.
#[derive(Debug, Default)]
struct RawData {
    bpmn_type: String,
    attributes: HashMap<Attrib, String>,
    symbol: Option<Symbol>,
    data_index: Option<usize>,
    outputs: Vec<String>,
    inputs: Vec<String>,
}

impl RawData {
    fn new(bpmn_type: impl Into<String>, attributes: HashMap<Attrib, String>) -> RawData {
        Self {
            bpmn_type: bpmn_type.into(),
            attributes,
            ..Default::default()
        }
    }

    fn name_or_id(&self) -> Result<&str, BpmnError> {
        Ok(self
            .attributes
            .get(&Attrib::Name)
            .or_else(|| self.attributes.get(&Attrib::Id))
            .ok_or_else(|| BpmnErrorKind::MissingId(self.bpmn_type.clone()))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_file() -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{:#?}",
            read_bpmn(quick_xml::Reader::from_file("examples/counter.bpmn")?)
        );
        Ok(())
    }
}
