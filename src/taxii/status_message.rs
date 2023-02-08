use xml::reader::{EventReader, XmlEvent};

use super::errors::MyError;

#[derive(Clone)]
pub struct StatusMessage {
    pub message_id: String,
    pub in_response_to: String,
    pub status_type: String, // TODO: this is probably an enum
    pub message: Option<String>,
}

impl StatusMessage {
    pub fn new_empty() -> StatusMessage {
        StatusMessage {
            message_id: String::from(""),
            in_response_to: String::from(""),
            status_type: String::from(""),
            message: None,
        }
    }
}

enum InTag {
    StatusMessage,
    Message,
}

pub fn parse_status_message(doc: &[u8]) -> Result<StatusMessage, MyError> {
    let mut tag_stack = Vec::<InTag>::new();
    let mut status_message = StatusMessage::new_empty();
    let mut last_value: String = String::new();
    let xml_parser = EventReader::new(doc);
    for e in xml_parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => match name.local_name.as_str() {
                "Status_Message" => {
                    if tag_stack.len() != 0 {
                        return Err(MyError(format!("unexpected tag preceeding Status_Message")));
                    }
                    tag_stack.push(InTag::StatusMessage);
                    for attr in attributes {
                        match attr.name.local_name.as_str() {
                            "message_id" => status_message.message_id = attr.value.clone(),
                            "in_response_to" => status_message.in_response_to = attr.value.clone(),
                            "status_type" => status_message.status_type = attr.value.clone(),
                            _ => {
                                return Err(MyError(format!(
                                    "unrecogized attribute: {}",
                                    attr.name.local_name
                                )))
                            }
                        }
                    }
                }
                "Message" => {
                    if tag_stack.len() != 1 {
                        return Err(MyError(format!("unexpected tag depth for Message")));
                    }
                    tag_stack.push(InTag::Message);
                }
                tag => {
                    return Err(MyError(format!("unexpected XML tag: {}", tag)));
                }
            },
            // TODO: seems like excessive cloning
            Ok(XmlEvent::EndElement { name }) => match tag_stack.pop() {
                Some(InTag::StatusMessage) => {
                    if name.local_name != "Status_Message" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                }
                Some(InTag::Message) => {
                    if name.local_name != "Message" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    status_message.message = Some(last_value.clone());
                }
                None => return Err(MyError(format!("unexpected end tag: {}", name.local_name))),
            },
            Ok(XmlEvent::CData(ref data)) => {
                last_value = data.clone();
            }
            Ok(XmlEvent::Characters(ref data)) => {
                last_value = data.clone();
            }
            Err(e) => {
                return Err(MyError(e.to_string()));
            }
            _ => {}
        }
    }
    Ok(status_message)
}

#[cfg(test)]
mod tests {
    use std::{env, fs::read_to_string, path::Path};

    use crate::taxii::status_message::parse_status_message;

    #[test]
    fn test_parse_status_message() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path =
            Path::new(path.as_str()).join("test/sample-status-message-response-bad-message.xml");
        let doc = read_to_string(path).unwrap();
        let status_message = parse_status_message(doc.as_bytes()).unwrap();
        assert_eq!("9125177396285394141", status_message.message_id);
        assert_eq!("0", status_message.in_response_to);
        assert_eq!("BAD_MESSAGE", status_message.status_type);
    }
}
