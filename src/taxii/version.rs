use rand::prelude::*;
use reqwest;
use uuid::Uuid;
use xml::writer::{EmitterConfig, EventWriter, XmlEvent};

use super::errors::MyError;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Version {
    V10,
    V11,
    V21,
}

static NAMESPACE_10: &'static str = "http://taxii.mitre.org/messages/taxii_xml_binding-1";
static NAMESPACE_11: &'static str = "http://taxii.mitre.org/messages/taxii_xml_binding-1.1";

// TODO: CONTENT_TYPE_10?
static CONTENT_TYPE_11: &'static str = "application/xml";
static CONTENT_TYPE_21: &'static str = "application/taxii+json;version=2.1";

// Version URN for the TAXII Services Specification 1.0
static SERVICES_VERSION_URN_10: &'static str = "urn:taxii.mitre.org:services:1.0";
// Version URN for the TAXII XML Message Binding Specification 1.0
static XML_BINDING_VERSION_URN_10: &'static str = "urn:taxii.mitre.org:message:xml:1.0";

// Version URN for the TAXII Services Specification 1.1
static SERVICES_VERSION_URN_11: &'static str = "urn:taxii.mitre.org:services:1.1";
// Version URN for the TAXII XML Message Binding Specification 1.1
static XML_BINDING_VERSION_URN_11: &'static str = "urn:taxii.mitre.org:message:xml:1.1";

// Version URN for the TAXII HTTP Protocol Binding Specification 1.0
// Note: not HTTP/1.0, but the 1.0 version of the TAXII binding to HTTP
static XML_BINDING_HTTP_10: &'static str = "urn:taxii.mitre.org:protocol:http:1.0";

// Version URN for the TAXII HTTPS Protocol Binding Specification 1.0
// Note: not HTTP/1.0, but the 1.0 version of the TAXII binding to HTTPS
static XML_BINDING_HTTPS_10: &'static str = "urn:taxii.mitre.org:protocol:https:1.0";

static DEFAULT_TAXII_PROTOCOL_URN: &'static str = "urn:taxii.mitre.org:protocol:http:1.0";
static DEFAULT_TAXII_SERVICES_URN: &'static str = "urn:taxii.mitre.org:services:1.1";

impl Version {
    pub fn xml_namespace(&self) -> &str {
        match self {
            Version::V10 => NAMESPACE_10,
            Version::V11 => NAMESPACE_11,
            _ => panic!("TODO: version does not support XML"),
        }
    }
    pub fn xml_binding_urn(&self) -> &str {
        match self {
            Version::V10 => XML_BINDING_VERSION_URN_10,
            Version::V11 => XML_BINDING_VERSION_URN_11,
            _ => panic!("TODO: version does not support XML"),
        }
    }
    pub fn content_type(&self) -> &str {
        match self {
            Version::V10 => panic!("TODO"),
            Version::V11 => CONTENT_TYPE_11,
            Version::V21 => CONTENT_TYPE_21,
        }
    }
    pub fn message_id(&self) -> String {
        match self {
            Version::V10 => {
                // TODO: is this expensive to create?
                let mut rng = thread_rng();
                let v: u64 = rng.gen();
                return v.to_string();
            }
            // TODO: the taxiistand example server uses what looks like a numeric representation
            // of a UUID. Should we?
            Version::V11 => {
                let id = Uuid::new_v4();
                return id.to_string();
            }
            _ => panic!("TODO: does V21 use message IDs?"),
        }
    }
}

pub fn write_xml<'a, E>(writer: &mut EventWriter<&mut Vec<u8>>, event: E) -> Result<(), MyError>
where
    E: Into<XmlEvent<'a>>,
{
    match writer.write(event) {
        Ok(_) => Ok(()),
        Err(err) => return Err(MyError(err.to_string())),
    }
}

pub fn write_xml_tag_with_data(
    writer: &mut EventWriter<&mut Vec<u8>>,
    tag: &str,
    data: &str,
) -> Result<(), MyError> {
    write_xml(writer, XmlEvent::start_element(tag))?;
    write_xml(writer, XmlEvent::characters(data))?;
    write_xml(writer, XmlEvent::end_element())?;
    Ok(())
}

fn create_simple_request_body(tag: &str, ver: Version) -> Result<String, MyError> {
    let mut buf_writer: Vec<u8> = Vec::with_capacity(128);
    let mut writer = EmitterConfig::new()
        .write_document_declaration(false)
        .perform_indent(true)
        .create_writer(&mut buf_writer);
    let msg_id = ver.message_id();
    let tag = format!("taxii_11:{}", tag);
    let elem = XmlEvent::start_element(tag.as_str())
        .attr("message_id", msg_id.as_str())
        .ns("taxii_11", ver.xml_namespace());
    match writer.write(elem) {
        Ok(_) => (),
        Err(err) => return Err(MyError(err.to_string())),
    }
    let end_elem = XmlEvent::end_element();
    match writer.write(end_elem) {
        Ok(_) => (),
        Err(err) => return Err(MyError(err.to_string())),
    }
    // TODO: better check on conversion than unwrap
    return Ok(String::from_utf8(buf_writer).unwrap());
}

pub fn create_discovery_request_body(ver: Version) -> Result<String, MyError> {
    create_simple_request_body("Discovery_Request", ver)
}

pub fn create_collection_information_request_body(ver: Version) -> Result<String, MyError> {
    create_simple_request_body("Collection_Information_Request", ver)
}

// TODO: the generic XML document defclaration fails when talking to test.taxiistand.com -- is
// that the typical behaviour for other TAXII servers?

pub fn taxii_request(url: &str, username: &str, password: &str, request_body: &str, ver: Version) {
    let client = reqwest::blocking::Client::new();
    let xml_binding_urn = ver.xml_binding_urn();
    let request = match client
        .post(url)
        .basic_auth(username, Some(password))
        .body(String::from(request_body))
        .header("Accept", ver.content_type())
        .header("Content-Type", ver.content_type())
        .header("X-TAXII-Accept", xml_binding_urn)
        .header("X-TAXII-Content-Type", xml_binding_urn)
        .header("X-TAXII-Protocol", DEFAULT_TAXII_PROTOCOL_URN)
        .header("X-TAXII-Services", DEFAULT_TAXII_SERVICES_URN)
        .build()
    {
        Ok(req) => {
            println!("{:?}", req);
            req
        }
        Err(err) => panic!("{}", err),
    };
    match client.execute(request) {
        Ok(resp) => {
            println!("resp={:?}", resp);
            println!("body={}", resp.text().unwrap())
        }
        Err(err) => panic!("{}", err),
    }
}

pub fn discovery_request(url: &str, username: &str, password: &str, ver: Version) {
    match create_discovery_request_body(ver) {
        Ok(request_body) => taxii_request(url, username, password, request_body.as_str(), ver),
        Err(err) => panic!("{}", err),
    };
}

// TODO: the request mechanism doesn't really belong in the "version" namespace
pub fn collection_information_request(url: &str, username: &str, password: &str, ver: Version) {
    match create_collection_information_request_body(ver) {
        Ok(request_body) => taxii_request(url, username, password, request_body.as_str(), ver),
        Err(err) => panic!("{}", err),
    };
}
