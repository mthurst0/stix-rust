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

/*
SUBSCRIBE SUBSCRIBE - Request a subscription to the named TAXII Data Collection
UNSUBSCRIBE UNSUBSCRIBE - Request cancellation of an existing subscription to the named
TAXII Data Collection
PAUSE PAUSE - Suspend delivery of content for the identified subscription
RESUME RESUME – Resume delivery of content for the identified subscription
STATUS STATUS - Request information on all subscriptions the requester has established
for the named TAXII Data Collection.
*/

#[derive(Debug, Copy, Clone, PartialEq)]
enum SubscribeAction {
    Subscribe,
    Unsubscribe,
    Pause,
    Resume,
    Status,
}

impl SubscribeAction {
    pub fn to_str(&self) -> &str {
        match self {
            SubscribeAction::Subscribe => "SUBSCRIBE",
            SubscribeAction::Unsubscribe => "UNSUBSCRIBE",
            SubscribeAction::Pause => "PAUSE",
            SubscribeAction::Resume => "RESUME",
            SubscribeAction::Status => "STATUS",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum ResponseType {
    Full,
    CountOnly,
}

impl ResponseType {
    pub fn to_str(&self) -> &str {
        match self {
            ResponseType::Full => "FULL",
            ResponseType::CountOnly => "COUNT_ONLY",
        }
    }
}

struct ContentBinding {
    binding_id: String,
    subtype_id: Option<String>,
}

struct SubscriptionParameters {
    reponse_type: ResponseType,
    content_bindings: Vec<ContentBinding>,
    query: Option<String>,
    query_format_id: Option<String>,
}

struct PushParameters {
    protocol_binding: String,
    address: String,
    message_binding: String,
}

// TODO: Extended Headers?
// TODO: <ds:Signature>

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

fn create_subscribe_request_body(
    ver: Version,
    action: SubscribeAction,
    collection_name: &str,
    subscription_id: Option<&str>,
    subscription_parameters: Option<&SubscriptionParameters>,
    push_parameters: Option<&PushParameters>,
) -> Result<String, MyError> {
    let mut buf_writer: Vec<u8> = Vec::with_capacity(128);
    let mut writer = EmitterConfig::new()
        .write_document_declaration(false)
        .perform_indent(true)
        .create_writer(&mut buf_writer);

    let msg_id = ver.message_id();
    let tag = format!("taxii_11:Subscription_Management_Request");
    let elem = XmlEvent::start_element(tag.as_str())
        .attr("action", action.to_str())
        .attr("message_id", msg_id.as_str())
        .attr("collection_name", collection_name)
        .ns("taxii_11", ver.xml_namespace());

    // <Subscription_Management_Request>
    write_xml(&mut writer, elem)?;

    if action != SubscribeAction::Subscribe && subscription_id.is_some() {
        // <Subscription_ID></Subscription_ID>
        write_xml_tag_with_data(
            &mut writer,
            "taxii_11:Subscription_ID",
            subscription_id.unwrap(),
        )?;
    }
    if action == SubscribeAction::Subscribe && subscription_id.is_some() {
        return Err(MyError(String::from(
            "unexpected subscription ID provided with subscribe action",
        )));
    }

    if action == SubscribeAction::Subscribe && subscription_parameters.is_some() {
        let subscription_parameters = subscription_parameters.unwrap();
        // <Subscription_Parameters>
        write_xml(
            &mut writer,
            XmlEvent::start_element("taxii_11:Subscription_Parameters"),
        )?;
        // <Response_Type></Response_Type>
        write_xml_tag_with_data(
            &mut writer,
            "taxii_11:Response_Type",
            subscription_parameters.reponse_type.to_str(),
        )?;
        {
            for content_binding in subscription_parameters.content_bindings.iter() {
                // <Content_Binding>
                write_xml(
                    &mut writer,
                    XmlEvent::start_element("taxii_11:Content_Binding")
                        .attr("binding_id", content_binding.binding_id.as_str()),
                )?;
                match &content_binding.subtype_id {
                    Some(subtype_id) => {
                        write_xml(
                            &mut writer,
                            XmlEvent::start_element("taxii_11:Subtype")
                                .attr("binding_id", &subtype_id.as_str()),
                        )?;
                        write_xml(&mut writer, XmlEvent::end_element())?;
                    }
                    None => (),
                }
                // </Content_Binding>
                write_xml(&mut writer, XmlEvent::end_element())?;
            }
            {
                match &subscription_parameters.query {
                    Some(query) => {
                        // <Query>
                        match &subscription_parameters.query_format_id {
                            Some(query_format_id) => {
                                write_xml(
                                    &mut writer,
                                    XmlEvent::start_element("taxii_11:Query")
                                        .attr("format_id", query_format_id.as_str()),
                                )?;
                            }
                            None => {
                                write_xml(&mut writer, XmlEvent::start_element("taxii_11:Query"))?
                            }
                        }
                        write_xml(&mut writer, XmlEvent::characters(query.as_str()))?;
                        // </Query>
                        write_xml(&mut writer, XmlEvent::end_element())?;
                    }
                    None => (),
                }
            }
        }
        // </Subscription_Parameters>
        write_xml(&mut writer, XmlEvent::end_element())?;
    }
    if action == SubscribeAction::Subscribe && push_parameters.is_some() {
        let push_parameters = push_parameters.unwrap();
        // <Push_Parameters>
        write_xml(
            &mut writer,
            XmlEvent::start_element("taxii_11:Push_Parameters"),
        )?;
        {
            // <Protocol_Binding></Protocol_Binding>
            write_xml_tag_with_data(
                &mut writer,
                "taxii_11:Protocol_Binding",
                &push_parameters.protocol_binding.as_str(),
            )?;
        }
        {
            // <Address></Address>
            write_xml_tag_with_data(
                &mut writer,
                "taxii_11:Address",
                &push_parameters.address.as_str(),
            )?;
        }
        {
            // <Message_Binding></Message_Binding>
            write_xml_tag_with_data(
                &mut writer,
                "taxii_11:Message_Binding",
                &push_parameters.message_binding.as_str(),
            )?;
        }
        // </Push_Parameters>
        write_xml(&mut writer, XmlEvent::end_element())?;
    }

    // </Subscription_Management_Request>
    write_xml(&mut writer, XmlEvent::end_element())?;
    // TODO: better check on conversion than unwrap
    return Ok(String::from_utf8(buf_writer).unwrap());
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

pub fn taxii_request(
    url: &str,
    username: &str,
    password: &str,
    request_body: &String,
    ver: Version,
) {
    println!("request_body: {}", request_body);
    let client = reqwest::blocking::Client::new();
    let xml_binding_urn = ver.xml_binding_urn();
    let request = match client
        .post(url)
        .basic_auth(username, Some(password))
        // TODO: unnecessary clone - remain befuddled by lifetimes
        .body(request_body.clone())
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
        Ok(v) => taxii_request(url, username, password, &v, ver),
        Err(err) => panic!("{}", err),
    };
}

// TODO: the request mechanism doesn't really belong in the "version" namespace
pub fn collection_information_request(url: &str, username: &str, password: &str, ver: Version) {
    match create_collection_information_request_body(ver) {
        Ok(v) => taxii_request(url, username, password, &v, ver),
        Err(err) => panic!("{}", err),
    };
}

#[cfg(test)]
mod tests {
    use super::{create_subscribe_request_body, SubscribeAction, Version};

    #[test]
    fn test_create_subscribe_request_body() {
        let result = create_subscribe_request_body(
            Version::V11,
            SubscribeAction::Subscribe,
            "collection-name-1",
            Some("subscription-id-1"),
            None,
            None,
        );
        assert!(result.is_err());

        let result = create_subscribe_request_body(
            Version::V11,
            SubscribeAction::Subscribe,
            "collection-name-1",
            None,
            None,
            None,
        );
        let result = result.unwrap();
        assert!(result.starts_with(
            "<taxii_11:Subscription_Management_Request xmlns:taxii_11=\"http://taxii.mitre.org/messages/taxii_xml_binding-1.1\" \
            action=\"SUBSCRIBE\" message_id="));
        assert!(result.ends_with("collection_name=\"collection-name-1\" />"));
    }
}
