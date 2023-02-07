use xml::reader::{EventReader, XmlEvent};

use super::errors::MyError;

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceType {
    Undefined,
    CollectionManagement,
    Discovery,
    Inbox,
    Poll,
}

impl ServiceType {
    pub fn parse(v: &str) -> Result<ServiceType, MyError> {
        match v {
            "COLLECTION_MANAGEMENT" => Ok(ServiceType::CollectionManagement),
            "DISCOVERY" => Ok(ServiceType::Discovery),
            "INBOX" => Ok(ServiceType::Inbox),
            "POLL" => Ok(ServiceType::Poll),
            _ => Err(MyError(format!("could not parse: {}", v))),
        }
    }
}

#[derive(Clone)]
pub struct ServiceInstance {
    pub service_type: ServiceType,
    pub service_version: String,
    pub available: bool,
    pub protocol_binding: String,
    pub address: String,
    pub message_bindings: Vec<String>,
    pub content_bindings: Vec<String>,
    pub message: Option<String>,
}

impl ServiceInstance {
    pub fn new_empty() -> ServiceInstance {
        ServiceInstance {
            service_type: ServiceType::Undefined,
            service_version: String::from(""),
            available: false,
            protocol_binding: String::from(""),
            address: String::from(""),
            message_bindings: Vec::<String>::new(),
            content_bindings: Vec::<String>::new(),
            message: None,
        }
    }
}

pub struct ServiceSet {
    services: Vec<ServiceInstance>,
}

impl ServiceSet {
    pub fn new() -> ServiceSet {
        return ServiceSet {
            services: Vec::<ServiceInstance>::new(),
        };
    }
}

fn indent(size: usize) -> String {
    const INDENT: &'static str = "    ";
    (0..size)
        .map(|_| INDENT)
        .fold(String::with_capacity(size * INDENT.len()), |r, s| r + s)
}

enum InTag {
    DiscoveryResponse,
    ServiceInstance,
    ProtocolBinding,
    Address,
    MessageBinding,
    ContentBinding,
    Message,
}

pub fn parse_discovery_response(doc: &[u8]) -> Result<ServiceSet, MyError> {
    let mut tag_stack = Vec::<InTag>::new();
    let mut services = ServiceSet::new();
    let mut cur_service = ServiceInstance::new_empty();
    let mut last_value: String = String::new();
    let xml_parser = EventReader::new(doc);
    for e in xml_parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                match name.local_name.as_str() {
                    "Discovery_Response" => {
                        if tag_stack.len() != 0 {
                            return Err(MyError(format!(
                                "unexpected tag preceeding Discovery_Response"
                            )));
                        }
                        tag_stack.push(InTag::DiscoveryResponse)
                    }
                    "Service_Instance" => {
                        if tag_stack.len() != 1 {
                            return Err(MyError(format!(
                                "unexpected tag depth for Service_Instance"
                            )));
                        }
                        tag_stack.push(InTag::ServiceInstance);
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "service_type" => {
                                    cur_service.service_type =
                                        match ServiceType::parse(attr.value.as_str()) {
                                            Ok(v) => v,
                                            Err(err) => panic!("{}", err), // TODO: return
                                        }
                                }
                                "service_version" => {
                                    cur_service.service_version = attr.value.clone()
                                }
                                "available" => {
                                    cur_service.available = attr.value.to_lowercase().eq("true")
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
                    "Protocol_Binding" => {
                        if tag_stack.len() != 2 {
                            return Err(MyError(format!(
                                "unexpected tag depth for Protocol_Binding"
                            )));
                        }
                        tag_stack.push(InTag::ProtocolBinding);
                    }
                    "Address" => {
                        if tag_stack.len() != 2 {
                            return Err(MyError(format!("unexpected tag depth for Address")));
                        }
                        tag_stack.push(InTag::Address);
                    }
                    "Message_Binding" => {
                        if tag_stack.len() != 2 {
                            return Err(MyError(format!(
                                "unexpected tag depth for Message_Binding"
                            )));
                        }
                        tag_stack.push(InTag::MessageBinding);
                    }
                    "Content_Binding" => {
                        if tag_stack.len() != 2 {
                            return Err(MyError(format!(
                                "unexpected tag depth for Content_Binding"
                            )));
                        }
                        tag_stack.push(InTag::ContentBinding);
                    }
                    "Message" => {
                        if tag_stack.len() != 2 {
                            return Err(MyError(format!("unexpected tag depth for Message")));
                        }
                        tag_stack.push(InTag::Message);
                    }
                    tag => {
                        return Err(MyError(format!("unexpected XML tag: {}", tag)));
                    }
                }
            }
            // TODO: seems like excessive cloning
            Ok(XmlEvent::EndElement { name }) => match tag_stack.pop() {
                // TODO: should we verify 'name' versus our tag stack?
                Some(InTag::DiscoveryResponse) => {
                    if name.local_name != "Discovery_Response" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                }
                Some(InTag::ServiceInstance) => {
                    if name.local_name != "Service_Instance" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    services.services.push(cur_service.clone());
                    cur_service = ServiceInstance::new_empty();
                }
                Some(InTag::ProtocolBinding) => {
                    if name.local_name != "Protocol_Binding" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    cur_service.protocol_binding = last_value.clone()
                }
                Some(InTag::Address) => {
                    if name.local_name != "Address" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    cur_service.address = last_value.clone()
                }
                Some(InTag::MessageBinding) => {
                    if name.local_name != "Message_Binding" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    cur_service.message_bindings.push(last_value.clone())
                }
                Some(InTag::ContentBinding) => {
                    if name.local_name != "Content_Binding" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    cur_service.content_bindings.push(last_value.clone())
                }
                Some(InTag::Message) => {
                    if name.local_name != "Message" {
                        return Err(MyError(format!("malformed XML response")));
                    }
                    cur_service.message = Some(last_value.clone())
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
    Ok(services)
}

#[cfg(test)]
mod tests {
    use crate::taxii::services::{parse_discovery_response, ServiceType};

    #[test]
    fn test_parse_discovery_response() {
        let doc = r###"
<taxii_11:Discovery_Response xmlns:taxii="http://taxii.mitre.org/messages/taxii_xml_binding-1" xmlns:taxii_11="http://taxii.mitre.org/messages/taxii_xml_binding-1.1" xmlns:tdq="http://taxii.mitre.org/query/taxii_default_query-1" message_id="4973304169178633585" in_response_to="32cacab7-bdd1-41d1-b5b1-02e3491fe9e8">
<taxii_11:Service_Instance service_type="INBOX" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/inbox-all</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Message>Test inbox, accepting all content.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="INBOX" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/inbox-stix</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Content_Binding binding_id="urn:stix.mitre.org:xml:1.0"/>
    <taxii_11:Content_Binding binding_id="urn:stix.mitre.org:xml:1.0.1"/>
    <taxii_11:Content_Binding binding_id="urn:stix.mitre.org:xml:1.1"/>
    <taxii_11:Content_Binding binding_id="urn:stix.mitre.org:xml:1.1.1"/>
    <taxii_11:Content_Binding binding_id="urn:stix.mitre.org:xml:1.2"/>
    <taxii_11:Message>Test inbox, accepting only STIX documents.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="INBOX" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/inbox-cap</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Content_Binding binding_id="urn:oasis:names:tc:emergency:cap:1.1"/>
    <taxii_11:Content_Binding binding_id="urn:oasis:names:tc:emergency:cap:1.2"/>
    <taxii_11:Message>Test inbox, accepting only CAP documents.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="INBOX" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/inbox-xmlenc</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Content_Binding binding_id="http://www.w3.org/2001/04/xmlenc#"/>
    <taxii_11:Message>Test inbox, accepting only Encrypted XML documents.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="INBOX" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/inbox-pkcs7</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Content_Binding binding_id="application/pkcs7-mime"/>
    <taxii_11:Message>Test inbox, accepting only S/MIME documents.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="POLL" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding >urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/poll</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Message>Test poll service, used for all feeds.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="COLLECTION_MANAGEMENT" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/collection-management</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Message>Test collection managment service.</taxii_11:Message>
</taxii_11:Service_Instance>
<taxii_11:Service_Instance service_type="DISCOVERY" service_version="urn:taxii.mitre.org:services:1.1" available="true">
    <taxii_11:Protocol_Binding>urn:taxii.mitre.org:protocol:https:1.0</taxii_11:Protocol_Binding>
    <taxii_11:Address>https://test.taxiistand.com/read-write/services/discovery</taxii_11:Address>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.0</taxii_11:Message_Binding>
    <taxii_11:Message_Binding>urn:taxii.mitre.org:message:xml:1.1</taxii_11:Message_Binding>
    <taxii_11:Message>Test discovery service.</taxii_11:Message>
</taxii_11:Service_Instance>
</taxii_11:Discovery_Response>
    "###;

        let services = parse_discovery_response(doc.as_bytes()).unwrap();
        assert_eq!(8, services.services.len());
        assert_eq!(ServiceType::Inbox, services.services[0].service_type);
        assert_eq!(ServiceType::Inbox, services.services[1].service_type);
        assert_eq!(ServiceType::Inbox, services.services[2].service_type);
        assert_eq!(ServiceType::Inbox, services.services[3].service_type);
        assert_eq!(ServiceType::Inbox, services.services[4].service_type);
        assert_eq!(ServiceType::Poll, services.services[5].service_type);
        assert_eq!(
            ServiceType::CollectionManagement,
            services.services[6].service_type
        );
        assert_eq!(ServiceType::Discovery, services.services[7].service_type);
        assert_eq!(
            services.services[0].address,
            "https://test.taxiistand.com/read-write/services/inbox-all"
        );
        assert_eq!(
            services.services[0].message.as_ref().unwrap().as_str(),
            "Test inbox, accepting all content."
        );
        assert_eq!(
            services.services[1].address,
            "https://test.taxiistand.com/read-write/services/inbox-stix"
        );
        assert_eq!(
            services.services[1].message.as_ref().unwrap().as_str(),
            "Test inbox, accepting only STIX documents."
        );
        assert_eq!(
            services.services[2].address,
            "https://test.taxiistand.com/read-write/services/inbox-cap"
        );
        assert_eq!(
            services.services[2].message.as_ref().unwrap().as_str(),
            "Test inbox, accepting only CAP documents."
        );
        assert_eq!(
            services.services[3].address,
            "https://test.taxiistand.com/read-write/services/inbox-xmlenc"
        );
        assert_eq!(
            services.services[3].message.as_ref().unwrap().as_str(),
            "Test inbox, accepting only Encrypted XML documents."
        );
        assert_eq!(
            services.services[4].address,
            "https://test.taxiistand.com/read-write/services/inbox-pkcs7"
        );
        assert_eq!(
            services.services[4].message.as_ref().unwrap().as_str(),
            "Test inbox, accepting only S/MIME documents."
        );
        assert_eq!(
            services.services[5].address,
            "https://test.taxiistand.com/read-write/services/poll"
        );
        assert_eq!(
            services.services[5].message.as_ref().unwrap().as_str(),
            "Test poll service, used for all feeds."
        );
        assert_eq!(
            services.services[6].address,
            "https://test.taxiistand.com/read-write/services/collection-management"
        );
        assert_eq!(
            services.services[6].message.as_ref().unwrap().as_str(),
            "Test collection managment service."
        );
        assert_eq!(
            services.services[7].address,
            "https://test.taxiistand.com/read-write/services/discovery"
        );
        assert_eq!(
            services.services[7].message.as_ref().unwrap().as_str(),
            "Test discovery service."
        );

        for service in services.services {
            assert_eq!(service.service_version, "urn:taxii.mitre.org:services:1.1");
            assert!(service.available);
            assert_eq!(
                service.message_bindings[0],
                "urn:taxii.mitre.org:message:xml:1.0"
            );
            assert_eq!(
                service.message_bindings[1],
                "urn:taxii.mitre.org:message:xml:1.1"
            );
            assert_eq!(
                service.protocol_binding,
                "urn:taxii.mitre.org:protocol:https:1.0"
            );
        }
    }
}
