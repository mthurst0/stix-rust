use chrono::{DateTime, Utc};
use xml::writer;

use super::{
    errors::MyError,
    types::{ContentBinding, ResponseType},
    version::{taxii_request, write_xml, write_xml_tag_with_data, Version},
};
struct TimeRange {
    exclusive_begin: Option<DateTime<Utc>>,
    inclusive_end: Option<DateTime<Utc>>,
}

struct DeliveryParameters {
    protocol_binding: String,
    address: String,
    message_binding: String,
}

struct PollParameters {
    allow_asynch: bool,
    response_type: ResponseType,
    content_bindings: Vec<ContentBinding>,
    query: String,
    query_format_id: String,
}

fn create_poll_request_body(
    ver: Version,
    collection_name: &str,
    time_range: Option<TimeRange>,
    subscription_id: &str,
    poll_paramters: Option<PollParameters>,
) -> Result<String, MyError> {
    let mut buf_writer: Vec<u8> = Vec::with_capacity(128);
    let mut writer = writer::EmitterConfig::new()
        .write_document_declaration(false)
        .perform_indent(true)
        .create_writer(&mut buf_writer);

    let msg_id = ver.message_id();
    let tag = format!("taxii_11:Poll_Request");
    let elem = writer::XmlEvent::start_element(tag.as_str())
        .attr("message_id", msg_id.as_str())
        .attr("collection_name", collection_name)
        .ns("taxii_11", ver.xml_namespace());

    // <Poll_Request>
    write_xml(&mut writer, elem)?;

    match time_range {
        Some(time_range) => match time_range.exclusive_begin {
            Some(exclusive_begin) => write_xml_tag_with_data(
                &mut writer,
                "taxii_11:Exclusive_Begin_Timestamp",
                exclusive_begin.to_rfc3339().as_str(),
            )?,
            _ => (),
        },
        None => (),
    }

    // TODO: time_range

    // <Subscription_ID></Subscription_ID>
    write_xml_tag_with_data(&mut writer, "taxii_11:Subscription_ID", subscription_id)?;

    // TODO: PollParameters

    // </PollRequest>
    write_xml(&mut writer, writer::XmlEvent::end_element())?;

    // TODO: better check on conversion than unwrap
    return Ok(String::from_utf8(buf_writer).unwrap());
}

pub fn poll_request(
    url: &str,
    username: &str,
    password: &str,
    ver: Version,
    collection_name: &str,
    subscription_id: &str,
) {
    // e.g.
    // let time_range = Some(TimeRange {
    // exclusive_begin: Utc::now().checked_sub_days(Days::new(1)),
    // inclusive_end: Some(Utc::now()),
    // });
    match create_poll_request_body(ver, collection_name, None, subscription_id, None) {
        Ok(request_body) => taxii_request(url, username, password, &request_body, ver),
        Err(err) => panic!("{}", err),
    }
}
