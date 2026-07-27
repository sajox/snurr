mod builder;

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
                let (line, column) = reader.line_and_column(&buf, true)?;
                return Err(ParseErrorKind::Xml {
                    line,
                    column,
                    source: e.into(),
                }
                .into());
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
                | OUTGOING
                | INCOMING
                | EXCLUSIVE_GATEWAY
                | PARALLEL_GATEWAY
                | INCLUSIVE_GATEWAY
                | EVENT_BASED_GATEWAY
                | SEQUENCE_FLOW) => builder.add(
                    Bpmn::try_from((bpmn_type, collect_attributes(&bs)))
                        .map_err(|e| create_parse_error(e, &reader, &buf))?,
                ),
                bpmn_type @ (DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION) => builder
                    .add_new_process(
                        Bpmn::try_from((bpmn_type, collect_attributes(&bs)))
                            .map_err(|e| create_parse_error(e, &reader, &buf))?,
                    ),
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
                        builder.add_to_process(
                            Bpmn::try_from((bpmn_type, collect_attributes(&bs)))
                                .map_err(|e| create_parse_error(e, &reader, &buf))?,
                        )?;
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
                | EVENT_BASED_GATEWAY
                | SEQUENCE_FLOW => builder.end()?,
                DEFINITIONS | PROCESS | SUB_PROCESS | TRANSACTION => builder.end_process()?,
                _ => {}
            },
            Ok(Event::Text(bt)) => {
                builder.add_text(
                    bt.decode()
                        .map_err(|e| ParseErrorKind::Encoding(e.into()))?
                        .into_owned(),
                );
            }
            // Ignore other XML events
            _ => (),
        }
    }
    Ok(builder.into())
}

fn collect_attributes<'a>(bs: &'a quick_xml::events::BytesStart<'_>) -> HashMap<&'a [u8], String> {
    bs.attributes()
        .filter_map(Result::ok)
        .filter_map(|attribute| {
            std::str::from_utf8(&attribute.value)
                .ok()
                .filter(|value| !value.is_empty())
                .map(|value| (attribute.key.local_name().into_inner(), value.into()))
        })
        .collect::<HashMap<&'a [u8], String>>()
}

fn create_parse_error<T>(source: BpmnError, reader: &Reader<T>, buf: &[u8]) -> ParseError {
    // This is not an XML error an thus set to false
    let result = reader.line_and_column(buf, false);
    if let Ok((line, _)) = result {
        ParseErrorKind::Bpmn { line, source }.into()
    } else {
        result.unwrap_err()
    }
}

trait LineAndColumn {
    fn line_and_column(&self, data: &[u8], xml_error: bool) -> Result<(usize, usize), ParseError>;
}

impl<T> LineAndColumn for Reader<T> {
    fn line_and_column(&self, data: &[u8], xml_error: bool) -> Result<(usize, usize), ParseError> {
        let end_pos = if xml_error {
            self.error_position()
        } else {
            self.buffer_position()
        } as usize;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_file() -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "{:#?}",
            read_bpmn(quick_xml::Reader::from_file("examples/example.bpmn")?)
        );
        Ok(())
    }
}
