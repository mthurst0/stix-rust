use xml::{reader, writer, EventReader};

use super::{
    errors::MyError,
    version::{taxii_request, write_xml, write_xml_tag_with_data, Version},
};

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
    pub fn parse(v: &str) -> Result<ResponseType, MyError> {
        match v {
            "FULL" => Ok(ResponseType::Full),
            "COUNT_ONLY" => Ok(ResponseType::CountOnly),
            _ => Err(MyError(format!("could not parse response type: {}", v))),
        }
    }
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

fn create_subscribe_request_body(
    ver: Version,
    action: SubscribeAction,
    collection_name: &str,
    subscription_id: Option<&str>,
    subscription_parameters: Option<&SubscriptionParameters>,
    push_parameters: Option<&PushParameters>,
) -> Result<String, MyError> {
    let mut buf_writer: Vec<u8> = Vec::with_capacity(128);
    let mut writer = writer::EmitterConfig::new()
        .write_document_declaration(false)
        .perform_indent(true)
        .create_writer(&mut buf_writer);

    let msg_id = ver.message_id();
    let tag = format!("taxii_11:Subscription_Management_Request");
    let elem = writer::XmlEvent::start_element(tag.as_str())
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
            writer::XmlEvent::start_element("taxii_11:Subscription_Parameters"),
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
                    writer::XmlEvent::start_element("taxii_11:Content_Binding")
                        .attr("binding_id", content_binding.binding_id.as_str()),
                )?;
                match &content_binding.subtype_id {
                    Some(subtype_id) => {
                        write_xml(
                            &mut writer,
                            writer::XmlEvent::start_element("taxii_11:Subtype")
                                .attr("binding_id", &subtype_id.as_str()),
                        )?;
                        write_xml(&mut writer, writer::XmlEvent::end_element())?;
                    }
                    None => (),
                }
                // </Content_Binding>
                write_xml(&mut writer, writer::XmlEvent::end_element())?;
            }
            {
                match &subscription_parameters.query {
                    Some(query) => {
                        // <Query>
                        match &subscription_parameters.query_format_id {
                            Some(query_format_id) => {
                                write_xml(
                                    &mut writer,
                                    writer::XmlEvent::start_element("taxii_11:Query")
                                        .attr("format_id", query_format_id.as_str()),
                                )?;
                            }
                            None => write_xml(
                                &mut writer,
                                writer::XmlEvent::start_element("taxii_11:Query"),
                            )?,
                        }
                        write_xml(&mut writer, writer::XmlEvent::characters(query.as_str()))?;
                        // </Query>
                        write_xml(&mut writer, writer::XmlEvent::end_element())?;
                    }
                    None => (),
                }
            }
        }
        // </Subscription_Parameters>
        write_xml(&mut writer, writer::XmlEvent::end_element())?;
    }
    if action == SubscribeAction::Subscribe && push_parameters.is_some() {
        let push_parameters = push_parameters.unwrap();
        // <Push_Parameters>
        write_xml(
            &mut writer,
            writer::XmlEvent::start_element("taxii_11:Push_Parameters"),
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
        write_xml(&mut writer, writer::XmlEvent::end_element())?;
    }

    // </Subscription_Management_Request>
    write_xml(&mut writer, writer::XmlEvent::end_element())?;
    // TODO: better check on conversion than unwrap
    return Ok(String::from_utf8(buf_writer).unwrap());
}

pub fn subscribe_request(
    url: &str,
    username: &str,
    password: &str,
    ver: Version,
    collection_name: &str,
) {
    match create_subscribe_request_body(
        ver,
        SubscribeAction::Subscribe,
        collection_name,
        None,
        None,
        None,
    ) {
        Ok(request_body) => taxii_request(url, username, password, &request_body, ver),
        Err(err) => panic!("{}", err),
    }
}

pub fn unsubscribe_request(
    url: &str,
    username: &str,
    password: &str,
    ver: Version,
    collection_name: &str,
    subscription_id: &str,
) {
    match create_subscribe_request_body(
        ver,
        SubscribeAction::Unsubscribe,
        collection_name,
        Some(subscription_id),
        None,
        None,
    ) {
        Ok(request_body) => taxii_request(url, username, password, &request_body, ver),
        Err(err) => panic!("{}", err),
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SubscriptionManagementResponseTag {
    SubscriptionManagementResponse,
    Subscription,
    SubscriptionID,
    SubscriptionParameters,
    ResponseType,
    PollInstance,
    ProtocolBinding,
    Address,
    MessageBinding,
}

impl SubscriptionManagementResponseTag {
    fn parse(tag: &str) -> Result<SubscriptionManagementResponseTag, MyError> {
        match tag {
            "Subscription_Management_Response" => {
                Ok(SubscriptionManagementResponseTag::SubscriptionManagementResponse)
            }
            "Subscription" => Ok(SubscriptionManagementResponseTag::Subscription),
            "Subscription_ID" => Ok(SubscriptionManagementResponseTag::SubscriptionID),
            "Subscription_Parameters" => {
                Ok(SubscriptionManagementResponseTag::SubscriptionParameters)
            }
            "Response_Type" => Ok(SubscriptionManagementResponseTag::ResponseType),
            "Poll_Instance" => Ok(SubscriptionManagementResponseTag::PollInstance),
            "Protocol_Binding" => Ok(SubscriptionManagementResponseTag::ProtocolBinding),
            "Address" => Ok(SubscriptionManagementResponseTag::Address),
            "Message_Binding" => Ok(SubscriptionManagementResponseTag::MessageBinding),
            _ => Err(MyError(format!("could not parse tag: {}", tag))),
        }
    }
    fn matches_expected_depth(&self, depth: usize) -> bool {
        match self {
            SubscriptionManagementResponseTag::SubscriptionManagementResponse => depth == 0,
            SubscriptionManagementResponseTag::Subscription => depth == 1,
            SubscriptionManagementResponseTag::SubscriptionID => depth == 2,
            SubscriptionManagementResponseTag::SubscriptionParameters => depth == 2,
            SubscriptionManagementResponseTag::ResponseType => depth == 3,
            SubscriptionManagementResponseTag::PollInstance => depth == 2,
            SubscriptionManagementResponseTag::ProtocolBinding => depth == 3,
            SubscriptionManagementResponseTag::Address => depth == 3,
            SubscriptionManagementResponseTag::MessageBinding => depth == 3,
        }
    }
    fn to_str(&self) -> &str {
        match self {
            SubscriptionManagementResponseTag::SubscriptionManagementResponse => {
                "Subscription_Management_Response"
            }
            SubscriptionManagementResponseTag::Subscription => "Subscription",
            SubscriptionManagementResponseTag::SubscriptionID => "Subscription_ID",
            SubscriptionManagementResponseTag::SubscriptionParameters => "Subscription_Parameters",
            SubscriptionManagementResponseTag::ResponseType => "Response_Type",
            SubscriptionManagementResponseTag::PollInstance => "Poll_Instance",
            SubscriptionManagementResponseTag::ProtocolBinding => "Protocol_Binding",
            SubscriptionManagementResponseTag::Address => "Address",
            SubscriptionManagementResponseTag::MessageBinding => "Message_Binding",
        }
    }
}

#[derive(Clone)]
struct PollInstance {
    protocol_binding: String,
    address: String,
    message_bindings: Vec<String>,
}

impl PollInstance {
    fn new_empty() -> PollInstance {
        return PollInstance {
            protocol_binding: String::from(""),
            address: String::from(""),
            message_bindings: Vec::<String>::new(),
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum SubscriptionStatus {
    Active,
    Paused,
    Unsubscribed,
}

impl SubscriptionStatus {
    pub fn parse(v: &str) -> Result<SubscriptionStatus, MyError> {
        match v {
            "ACTIVE" => Ok(SubscriptionStatus::Active),
            "PAUSED" => Ok(SubscriptionStatus::Paused),
            "UNSUBSCRIBED" => Ok(SubscriptionStatus::Unsubscribed),
            _ => Err(MyError(format!(
                "could not parse subscription status: {}",
                v
            ))),
        }
    }
}

struct Subscription {
    status: SubscriptionStatus,
    id: String,
    response_type: ResponseType,
    poll_instances: Vec<PollInstance>,
    collection_name: String,
}

impl Subscription {
    pub fn new_empty() -> Subscription {
        return Subscription {
            status: SubscriptionStatus::Active,
            id: String::from(""),
            response_type: ResponseType::Full,
            poll_instances: Vec::<PollInstance>::new(),
            collection_name: String::from(""),
        };
    }
}

pub struct SubscriptionResponse {
    message_id: String,
    in_response_to: String,
    subscription: Subscription,
}

impl SubscriptionResponse {
    fn new_empty() -> SubscriptionResponse {
        return SubscriptionResponse {
            message_id: String::from(""),
            in_response_to: String::from(""),
            subscription: Subscription::new_empty(),
        };
    }
}

// TODO: test that we ignore a treat tags with 1 cardinality as errors,
// e.g. that there is only one <Subscription> tag.

pub fn parse_subscription_management_response(doc: &[u8]) -> Result<SubscriptionResponse, MyError> {
    let mut tag_stack = Vec::<SubscriptionManagementResponseTag>::new();
    let mut subscription_response = SubscriptionResponse::new_empty();
    let mut cur_poll_instance: Option<PollInstance> = None;
    let mut last_value: String = String::new();
    let xml_parser = EventReader::new(doc);
    for e in xml_parser {
        match e {
            Ok(reader::XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                let tag = match SubscriptionManagementResponseTag::parse(name.local_name.as_str()) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };
                if !tag.matches_expected_depth(tag_stack.len()) {
                    return Err(MyError(format!(
                        "tag at unexpected depth of {} expected: {}",
                        tag_stack.len(),
                        name.local_name.as_str()
                    )));
                }
                tag_stack.push(tag);
                match tag {
                    SubscriptionManagementResponseTag::SubscriptionManagementResponse => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "message_id" => {
                                    subscription_response.message_id = attr.value.clone()
                                }
                                "in_response_to" => {
                                    subscription_response.in_response_to = attr.value.clone()
                                }
                                "collection_name" => {
                                    subscription_response.subscription.collection_name =
                                        attr.value.clone();
                                }
                                "xmlns:taxii" | "xmlns:taxii_11" | "xmlns:tdq" => {
                                    // TODO: ignored for now
                                }
                                _ => {
                                    return Err(MyError(format!(
                                        "unrecogized attribute: {}",
                                        attr.name.local_name
                                    )))
                                }
                            }
                        }
                    }
                    SubscriptionManagementResponseTag::Subscription => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "status" => {
                                    subscription_response.subscription.status =
                                        match SubscriptionStatus::parse(attr.value.as_str()) {
                                            Ok(status) => status,
                                            Err(err) => return Err(MyError(err.to_string())),
                                        }
                                }
                                _ => {
                                    return Err(MyError(format!(
                                        "unrecogized attribute: {}",
                                        attr.name.local_name
                                    )))
                                }
                            }
                        }
                    }
                    SubscriptionManagementResponseTag::PollInstance => {
                        cur_poll_instance = Some(PollInstance::new_empty())
                    }
                    // We only match on tags that we need to parse attributes from. This default
                    // match is therefore: keep calm and carry on.
                    _ => (),
                }
            }
            Ok(reader::XmlEvent::EndElement { name }) => {
                let end_tag = SubscriptionManagementResponseTag::parse(name.local_name.as_str())?;
                let tag = tag_stack.pop();
                if tag.is_none() || tag.unwrap() != end_tag {
                    return Err(MyError(format!("malformed XML response")));
                }
                match end_tag {
                    SubscriptionManagementResponseTag::SubscriptionID => {
                        subscription_response.subscription.id = last_value.clone()
                    }
                    SubscriptionManagementResponseTag::ResponseType => {
                        subscription_response.subscription.response_type =
                            ResponseType::parse(last_value.as_str())?
                    }
                    SubscriptionManagementResponseTag::PollInstance => match cur_poll_instance {
                        Some(ref poll_instance) => subscription_response
                            .subscription
                            .poll_instances
                            .push(poll_instance.clone()),
                        None => {
                            return Err(MyError(format!("unexpected end tag for Poll_Instance")))
                        }
                    },
                    SubscriptionManagementResponseTag::ProtocolBinding => match cur_poll_instance {
                        Some(ref mut v) => v.protocol_binding = last_value.clone(),
                        None => return Err(MyError(format!("unexpected Protocol_Binding tag"))),
                    },
                    SubscriptionManagementResponseTag::Address => match cur_poll_instance {
                        Some(ref mut v) => v.address = last_value.clone(),
                        None => return Err(MyError(format!("unexpected Address tag"))),
                    },
                    SubscriptionManagementResponseTag::MessageBinding => match cur_poll_instance {
                        Some(ref mut v) => v.message_bindings.push(last_value.clone()),
                        None => return Err(MyError(format!("unexpected Address tag"))),
                    },
                    _ => (),
                }
            }
            Ok(reader::XmlEvent::Characters(ref data)) => {
                last_value = data.clone();
            }
            Err(e) => {
                return Err(MyError(e.to_string()));
            }
            _ => {}
        }
    }
    Ok(subscription_response)
}

#[cfg(test)]
mod tests {
    use crate::taxii::subscriptions::{ResponseType, SubscriptionStatus};

    use super::{
        create_subscribe_request_body, parse_subscription_management_response, SubscribeAction,
        Version,
    };
    use std::{env, fs::read_to_string, path::Path};

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

    #[test]
    fn test_parse_subscription_management_response_subscribe() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(path.as_str())
            .join("test/sample-subscription-management-response-subscribe.xml");
        let doc = read_to_string(path).unwrap();
        let subscription_response = match parse_subscription_management_response(doc.as_bytes()) {
            Ok(v) => v,
            Err(err) => panic!("test failed: {}", err),
        };
        assert_eq!("3326595023702419548", subscription_response.message_id);
        assert_eq!(
            "ec5e5744-5b91-4533-adbc-be2d1a1cf160",
            subscription_response.in_response_to
        );
        let sub = subscription_response.subscription;
        assert_eq!("stix-data", sub.collection_name);
        assert_eq!(SubscriptionStatus::Active, sub.status);
        assert_eq!("8954140241256270840", sub.id);
        assert_eq!(ResponseType::Full, sub.response_type);
        assert_eq!(2, sub.poll_instances.len());
        assert_eq!(
            "urn:taxii.mitre.org:protocol:https:1.0",
            sub.poll_instances[0].protocol_binding
        );
        assert_eq!(
            "https://test.taxiistand.com/read-write/services/poll",
            sub.poll_instances[0].address
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.0",
            sub.poll_instances[0].message_bindings[0]
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.1",
            sub.poll_instances[0].message_bindings[1]
        );

        assert_eq!(
            "urn:taxii.mitre.org:protocol:https:1.0",
            sub.poll_instances[1].protocol_binding
        );
        assert_eq!(
            "https://test.taxiistand.com/read-write-auth/services/poll",
            sub.poll_instances[1].address
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.0",
            sub.poll_instances[1].message_bindings[0]
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.1",
            sub.poll_instances[1].message_bindings[1]
        );
    }

    #[test]
    fn test_parse_subscription_management_response_unsubscribe() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(path.as_str())
            .join("test/sample-subscription-management-response-unsubscribe.xml");
        let doc = read_to_string(path).unwrap();
        let subscription_response = match parse_subscription_management_response(doc.as_bytes()) {
            Ok(v) => v,
            Err(err) => panic!("test failed: {}", err),
        };
        assert_eq!("3214749113040463214", subscription_response.message_id);
        assert_eq!(
            "3135d61d-d990-4706-b394-9b441d4f2d3f",
            subscription_response.in_response_to
        );
        let sub = subscription_response.subscription;
        assert_eq!("stix-data", sub.collection_name);
        assert_eq!(SubscriptionStatus::Unsubscribed, sub.status);
        assert_eq!("8954140241256270840", sub.id);
        assert_eq!(ResponseType::Full, sub.response_type);
        assert_eq!(2, sub.poll_instances.len());
        assert_eq!(
            "urn:taxii.mitre.org:protocol:https:1.0",
            sub.poll_instances[0].protocol_binding
        );
        assert_eq!(
            "https://test.taxiistand.com/read-write/services/poll",
            sub.poll_instances[0].address
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.0",
            sub.poll_instances[0].message_bindings[0]
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.1",
            sub.poll_instances[0].message_bindings[1]
        );

        assert_eq!(
            "urn:taxii.mitre.org:protocol:https:1.0",
            sub.poll_instances[1].protocol_binding
        );
        assert_eq!(
            "https://test.taxiistand.com/read-write-auth/services/poll",
            sub.poll_instances[1].address
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.0",
            sub.poll_instances[1].message_bindings[0]
        );
        assert_eq!(
            "urn:taxii.mitre.org:message:xml:1.1",
            sub.poll_instances[1].message_bindings[1]
        );
    }
}
