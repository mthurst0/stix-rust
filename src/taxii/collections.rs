use xml::reader::{EventReader, XmlEvent};

use super::errors::MyError;

#[derive(Debug, Clone, PartialEq)]
pub enum CollectionType {
    Unknown,
    DataFeed,
}

impl CollectionType {
    pub fn parse(v: &str) -> Result<CollectionType, MyError> {
        match v {
            "DATA_FEED" => Ok(CollectionType::DataFeed),
            _ => Err(MyError(format!("could not parse: {}", v))),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CollectionServiceType {
    PollingService,
    SubscriptionService,
    ReceivingInboxService,
}

#[derive(Clone)]
pub struct CollectionService {
    pub collection_service_type: CollectionServiceType,
    pub protocol_binding: String,
    pub address: String,
    pub message_bindings: Vec<String>,
    pub content_bindings: Vec<String>,
}

impl CollectionService {
    pub fn new(collection_service_type: CollectionServiceType) -> CollectionService {
        return CollectionService {
            collection_service_type,
            protocol_binding: String::from(""),
            address: String::from(""),
            message_bindings: Vec::<String>::new(),
            content_bindings: Vec::<String>::new(),
        };
    }
}

#[derive(Clone)]
pub struct Collection {
    pub collection_name: String,
    pub collection_type: CollectionType,
    pub available: bool,
    pub description: String,
    pub collection_volume: String,
    pub content_bindings: Vec<String>,
    pub collection_services: Vec<CollectionService>,
}

impl Collection {
    pub fn new_empty() -> Collection {
        Collection {
            collection_name: String::from(""),
            collection_type: CollectionType::Unknown,
            available: false,
            description: String::from(""),
            collection_volume: String::from(""),
            content_bindings: Vec::<String>::new(),
            collection_services: Vec::<CollectionService>::new(),
        }
    }
}

pub struct CollectionSet {
    collections: Vec<Collection>,
}

impl CollectionSet {
    pub fn new() -> CollectionSet {
        return CollectionSet {
            collections: Vec::<Collection>::new(),
        };
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum CollectionTags {
    CollectionInformationResponse,
    Collection,
    Description,
    CollectionVolume,
    ContentBinding,
    MessageBinding,
    Address,
    ProtocolBinding,
    PollingService,
    SubscriptionService,
    ReceivingInboxService,
}

impl CollectionTags {
    fn parse(tag: &str) -> Result<CollectionTags, MyError> {
        match tag {
            "Collection_Information_Response" => Ok(CollectionTags::CollectionInformationResponse),
            "Collection" => Ok(CollectionTags::Collection),
            "Description" => Ok(CollectionTags::Description),
            "Collection_Volume" => Ok(CollectionTags::CollectionVolume),
            "Content_Binding" => Ok(CollectionTags::ContentBinding),
            "Message_Binding" => Ok(CollectionTags::MessageBinding),
            "Address" => Ok(CollectionTags::Address),
            "Protocol_Binding" => Ok(CollectionTags::ProtocolBinding),
            "Polling_Service" => Ok(CollectionTags::PollingService),
            "Subscription_Service" => Ok(CollectionTags::SubscriptionService),
            "Receiving_Inbox_Service" => Ok(CollectionTags::ReceivingInboxService),
            _ => Err(MyError(format!("could not parse tag: {}", tag))),
        }
    }
    fn matches_expected_depth(&self, depth: usize) -> bool {
        match self {
            CollectionTags::CollectionInformationResponse => depth == 0,
            CollectionTags::Collection => depth == 1,
            CollectionTags::Description => depth == 2,
            CollectionTags::CollectionVolume => depth == 2,
            CollectionTags::ContentBinding => depth == 2 || depth == 3,
            CollectionTags::MessageBinding => depth == 3,
            CollectionTags::Address => depth == 3,
            CollectionTags::ProtocolBinding => depth == 3,
            CollectionTags::PollingService => depth == 2,
            CollectionTags::SubscriptionService => depth == 2,
            CollectionTags::ReceivingInboxService => depth == 2,
        }
    }
    fn to_str(&self) -> &str {
        match self {
            CollectionTags::CollectionInformationResponse => "Collection_Information_Response",
            CollectionTags::Collection => "Collection",
            CollectionTags::Description => "Description",
            CollectionTags::CollectionVolume => "Collection_Volume",
            CollectionTags::ContentBinding => "Content_Binding",
            CollectionTags::MessageBinding => "Message_Binding",
            CollectionTags::Address => "Address",
            CollectionTags::ProtocolBinding => "Protocol_Binding",
            CollectionTags::PollingService => "Polling_Service",
            CollectionTags::SubscriptionService => "Subscription_Service",
            CollectionTags::ReceivingInboxService => "Receiving_DataFeed_Service",
        }
    }
}

pub fn parse_collection_information_response(doc: &[u8]) -> Result<CollectionSet, MyError> {
    let mut tag_stack = Vec::<CollectionTags>::new();
    let mut collection_set = CollectionSet::new();
    let mut cur_collection = Collection::new_empty();
    let mut cur_service: Option<CollectionService> = None;
    let mut last_value: String = String::new();
    let xml_parser = EventReader::new(doc);
    for e in xml_parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                let tag = match CollectionTags::parse(name.local_name.as_str()) {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };
                if !tag.matches_expected_depth(tag_stack.len()) {
                    return Err(MyError(format!(
                        "tag at unexpected depth of {}: {}",
                        tag_stack.len(),
                        name.local_name.as_str()
                    )));
                }
                tag_stack.push(tag);
                match tag {
                    CollectionTags::Collection => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "collection_name" => {
                                    cur_collection.collection_name = attr.value.clone()
                                }
                                "collection_type" => {
                                    cur_collection.collection_type =
                                        CollectionType::parse(attr.value.as_str())?
                                }
                                "available" => {
                                    cur_collection.available = attr.value.to_lowercase().eq("true")
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
                    CollectionTags::ContentBinding => {
                        for attr in attributes {
                            match attr.name.local_name.as_str() {
                                "binding_id" => match cur_service {
                                    Some(ref mut v) => v.content_bindings.push(attr.value.clone()),
                                    None => {
                                        cur_collection.content_bindings.push(attr.value.clone())
                                    }
                                },
                                _ => {
                                    return Err(MyError(format!(
                                        "unrecogized attribute: {}",
                                        attr.name.local_name
                                    )))
                                }
                            }
                        }
                    }
                    CollectionTags::PollingService => {
                        cur_service = Some(CollectionService::new(
                            CollectionServiceType::PollingService,
                        ));
                    }
                    CollectionTags::SubscriptionService => {
                        cur_service = Some(CollectionService::new(
                            CollectionServiceType::SubscriptionService,
                        ));
                    }
                    CollectionTags::ReceivingInboxService => {
                        cur_service = Some(CollectionService::new(
                            CollectionServiceType::ReceivingInboxService,
                        ));
                    }
                    // We only match on tags that we need to parse attributes from. This default
                    // match is therefore: keep calm and carry on.
                    _ => (),
                }
            }
            Ok(XmlEvent::EndElement { name }) => {
                let end_tag = CollectionTags::parse(name.local_name.as_str())?;
                let tag = tag_stack.pop();
                if tag.is_none() || tag.unwrap() != end_tag {
                    return Err(MyError(format!("malformed XML response")));
                }
                match end_tag {
                    CollectionTags::CollectionInformationResponse => {}
                    CollectionTags::Collection => {
                        collection_set.collections.push(cur_collection.clone());
                        cur_collection = Collection::new_empty();
                    }
                    CollectionTags::Description => {
                        cur_collection.description = last_value.clone();
                    }
                    CollectionTags::CollectionVolume => {
                        cur_collection.collection_volume = last_value.clone();
                    }
                    CollectionTags::ContentBinding => {
                        // Nothing to do, that attributes are extracted in the StartElement handlers
                    }
                    CollectionTags::MessageBinding => match cur_service {
                        Some(ref mut v) => v.message_bindings.push(last_value.clone()),
                        None => return Err(MyError(format!("unexpected Address tag"))),
                    },
                    CollectionTags::Address => match cur_service {
                        Some(ref mut v) => v.address = last_value.clone(),
                        None => return Err(MyError(format!("unexpected Address tag"))),
                    },
                    CollectionTags::ProtocolBinding => match cur_service {
                        Some(ref mut v) => v.protocol_binding = last_value.clone(),
                        None => return Err(MyError(format!("unexpected Protocol_Binding tag"))),
                    },
                    CollectionTags::PollingService
                    | CollectionTags::SubscriptionService
                    | CollectionTags::ReceivingInboxService => {
                        match cur_service {
                            Some(v) => cur_collection.collection_services.push(v.clone()),
                            None => return Err(MyError(format!("unexpected end tag for service"))),
                        }
                        cur_service = None
                    }
                }
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
    Ok(collection_set)
}

#[cfg(test)]
mod tests {
    use std::{env, fs::read_to_string, path::Path};

    use crate::taxii::collections::{
        parse_collection_information_response, CollectionServiceType, CollectionType,
    };

    #[test]
    fn test_parse_collection_information_response() {
        let path = env::var("CARGO_MANIFEST_DIR").unwrap();
        let path = Path::new(path.as_str()).join("test/sample-collection-information-response.xml");
        let doc = read_to_string(path).unwrap();
        let collection_set = match parse_collection_information_response(doc.as_bytes()) {
            Ok(v) => v,
            Err(err) => panic!("test failed: {}", err),
        };
        assert_eq!(5, collection_set.collections.len());
        for collection in collection_set.collections.iter() {
            assert_eq!(collection.available, true);
            assert_eq!(collection.collection_type, CollectionType::DataFeed);
        }
        // collection[0]
        {
            let collection0 = &collection_set.collections[0];
            assert_eq!("any-data", collection0.collection_name);
            assert_eq!(
                "This feed can contain/accept any data, with any binding, removed daily.",
                collection0.description
            );
            assert_eq!(0, collection0.content_bindings.len());
            assert_eq!("38217521", collection0.collection_volume);
            assert_eq!(6, collection0.collection_services.len());
            let expected0_service_types = [
                CollectionServiceType::PollingService,
                CollectionServiceType::PollingService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::ReceivingInboxService,
                CollectionServiceType::ReceivingInboxService,
            ];
            let expected0_addresses = [
                "https://test.taxiistand.com/read-write/services/poll",
                "https://test.taxiistand.com/read-write-auth/services/poll",
                "https://test.taxiistand.com/read-write/services/collection-management",
                "https://test.taxiistand.com/read-write-auth/services/collection-management",
                "https://test.taxiistand.com/read-write/services/inbox-all",
                "https://test.taxiistand.com/read-write-auth/services/inbox-all",
            ];
            for (pos, expected_address) in expected0_addresses.iter().enumerate() {
                assert_eq!(
                    expected0_service_types[pos],
                    collection0.collection_services[pos].collection_service_type
                );
                assert_eq!(
                    expected0_addresses[pos],
                    collection0.collection_services[pos].address
                );
                assert_eq!(
                    "urn:taxii.mitre.org:protocol:https:1.0",
                    collection0.collection_services[pos].protocol_binding
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.0",
                    collection0.collection_services[pos].message_bindings[0]
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.1",
                    collection0.collection_services[pos].message_bindings[1]
                );
            }
        }
        // collection[1]
        {
            let collection1 = &collection_set.collections[1];
            assert_eq!("stix-data", collection1.collection_name);
            assert_eq!(
                "This feed only contains/accepts STIX data, removed daily.",
                collection1.description
            );
            assert_eq!(5, collection1.content_bindings.len());
            let expected_collection1_content_bindings = [
                "urn:stix.mitre.org:xml:1.0",
                "urn:stix.mitre.org:xml:1.0.1",
                "urn:stix.mitre.org:xml:1.1",
                "urn:stix.mitre.org:xml:1.1.1",
                "urn:stix.mitre.org:xml:1.2",
            ];
            for pos in 0..expected_collection1_content_bindings.len() {
                assert_eq!(
                    expected_collection1_content_bindings[pos],
                    collection1.content_bindings[pos]
                );
            }
            let expected1_service_types = [
                CollectionServiceType::PollingService,
                CollectionServiceType::PollingService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::ReceivingInboxService,
                CollectionServiceType::ReceivingInboxService,
            ];
            let expected1_addresses = [
                "https://test.taxiistand.com/read-write/services/poll",
                "https://test.taxiistand.com/read-write-auth/services/poll",
                "https://test.taxiistand.com/read-write/services/collection-management",
                "https://test.taxiistand.com/read-write-auth/services/collection-management",
                "https://test.taxiistand.com/read-write/services/inbox-stix",
                "https://test.taxiistand.com/read-write-auth/services/inbox-stix",
            ];
            assert_eq!("13624", collection1.collection_volume);
            assert_eq!(6, collection1.collection_services.len());
            for pos in 0..6 {
                assert_eq!(
                    expected1_service_types[pos],
                    collection1.collection_services[pos].collection_service_type
                );
                assert_eq!(
                    expected1_addresses[pos],
                    collection1.collection_services[pos].address
                );
                assert_eq!(
                    "urn:taxii.mitre.org:protocol:https:1.0",
                    collection1.collection_services[pos].protocol_binding
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.0",
                    collection1.collection_services[pos].message_bindings[0]
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.1",
                    collection1.collection_services[pos].message_bindings[1]
                );
            }
            for pos in 0..expected_collection1_content_bindings.len() {
                assert_eq!(
                    expected_collection1_content_bindings[pos],
                    collection1.collection_services[4].content_bindings[pos]
                );
                assert_eq!(
                    expected_collection1_content_bindings[pos],
                    collection1.collection_services[5].content_bindings[pos]
                );
            }
        }
        // collection[2]
        {
            let collection2 = &collection_set.collections[2];
            assert_eq!("cap-data", collection2.collection_name);
            assert_eq!(
                "This feed only contains/accepts CAP data, removed daily.",
                collection2.description
            );
            assert_eq!(2, collection2.content_bindings.len());
            let expected_collection2_content_bindings = [
                "urn:oasis:names:tc:emergency:cap:1.1",
                "urn:oasis:names:tc:emergency:cap:1.2",
            ];
            for pos in 0..expected_collection2_content_bindings.len() {
                assert_eq!(
                    expected_collection2_content_bindings[pos],
                    collection2.content_bindings[pos]
                );
            }
            let expected2_service_types = [
                CollectionServiceType::PollingService,
                CollectionServiceType::PollingService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::ReceivingInboxService,
                CollectionServiceType::ReceivingInboxService,
            ];
            let expected2_addresses = [
                "https://test.taxiistand.com/read-write/services/poll",
                "https://test.taxiistand.com/read-write-auth/services/poll",
                "https://test.taxiistand.com/read-write/services/collection-management",
                "https://test.taxiistand.com/read-write-auth/services/collection-management",
                "https://test.taxiistand.com/read-write/services/inbox-cap",
                "https://test.taxiistand.com/read-write-auth/services/inbox-cap",
            ];
            assert_eq!("139", collection2.collection_volume);
            assert_eq!(6, collection2.collection_services.len());
            for pos in 0..6 {
                assert_eq!(
                    expected2_service_types[pos],
                    collection2.collection_services[pos].collection_service_type
                );
                assert_eq!(
                    expected2_addresses[pos],
                    collection2.collection_services[pos].address
                );
                assert_eq!(
                    "urn:taxii.mitre.org:protocol:https:1.0",
                    collection2.collection_services[pos].protocol_binding
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.0",
                    collection2.collection_services[pos].message_bindings[0]
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.1",
                    collection2.collection_services[pos].message_bindings[1]
                );
            }
            for pos in 0..expected_collection2_content_bindings.len() {
                assert_eq!(
                    expected_collection2_content_bindings[pos],
                    collection2.collection_services[4].content_bindings[pos]
                );
                assert_eq!(
                    expected_collection2_content_bindings[pos],
                    collection2.collection_services[5].content_bindings[pos]
                );
            }
        }
        // collection[3]
        {
            let collection3 = &collection_set.collections[3];
            assert_eq!("xmlenc-data", collection3.collection_name);
            assert_eq!(
                "This feed only contains/accepts Encrypted XML data, removed daily.",
                collection3.description
            );
            assert_eq!(1, collection3.content_bindings.len());
            let expected_collection3_content_bindings = ["http://www.w3.org/2001/04/xmlenc#"];
            for pos in 0..expected_collection3_content_bindings.len() {
                assert_eq!(
                    expected_collection3_content_bindings[pos],
                    collection3.content_bindings[pos]
                );
            }
            let expected3_service_types = [
                CollectionServiceType::PollingService,
                CollectionServiceType::PollingService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::ReceivingInboxService,
                CollectionServiceType::ReceivingInboxService,
            ];
            let expected3_addresses = [
                "https://test.taxiistand.com/read-write/services/poll",
                "https://test.taxiistand.com/read-write-auth/services/poll",
                "https://test.taxiistand.com/read-write/services/collection-management",
                "https://test.taxiistand.com/read-write-auth/services/collection-management",
                "https://test.taxiistand.com/read-write/services/inbox-xmlenc",
                "https://test.taxiistand.com/read-write-auth/services/inbox-xmlenc",
            ];
            assert_eq!("132", collection3.collection_volume);
            assert_eq!(6, collection3.collection_services.len());
            for pos in 0..6 {
                assert_eq!(
                    expected3_service_types[pos],
                    collection3.collection_services[pos].collection_service_type
                );
                assert_eq!(
                    expected3_addresses[pos],
                    collection3.collection_services[pos].address
                );
                assert_eq!(
                    "urn:taxii.mitre.org:protocol:https:1.0",
                    collection3.collection_services[pos].protocol_binding
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.0",
                    collection3.collection_services[pos].message_bindings[0]
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.1",
                    collection3.collection_services[pos].message_bindings[1]
                );
            }
            for pos in 0..expected_collection3_content_bindings.len() {
                assert_eq!(
                    expected_collection3_content_bindings[pos],
                    collection3.collection_services[4].content_bindings[pos]
                );
                assert_eq!(
                    expected_collection3_content_bindings[pos],
                    collection3.collection_services[5].content_bindings[pos]
                );
            }
        }
        // collection[4]
        {
            let collection4 = &collection_set.collections[4];
            assert_eq!("pkcs7-data", collection4.collection_name);
            assert_eq!(
                "This feed only contains/accpets S/MIME data, removed daily.",
                collection4.description
            );
            assert_eq!(1, collection4.content_bindings.len());
            let expected_collection4_content_bindings = ["application/pkcs7-mime"];
            for pos in 0..expected_collection4_content_bindings.len() {
                assert_eq!(
                    expected_collection4_content_bindings[pos],
                    collection4.content_bindings[pos]
                );
            }
            let expected4_service_types = [
                CollectionServiceType::PollingService,
                CollectionServiceType::PollingService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::SubscriptionService,
                CollectionServiceType::ReceivingInboxService,
                CollectionServiceType::ReceivingInboxService,
            ];
            let expected4_addresses = [
                "https://test.taxiistand.com/read-write/services/poll",
                "https://test.taxiistand.com/read-write-auth/services/poll",
                "https://test.taxiistand.com/read-write/services/collection-management",
                "https://test.taxiistand.com/read-write-auth/services/collection-management",
                "https://test.taxiistand.com/read-write/services/inbox-pkcs7",
                "https://test.taxiistand.com/read-write-auth/services/inbox-pkcs7",
            ];
            assert_eq!("134", collection4.collection_volume);
            assert_eq!(6, collection4.collection_services.len());
            for pos in 0..6 {
                assert_eq!(
                    expected4_service_types[pos],
                    collection4.collection_services[pos].collection_service_type
                );
                assert_eq!(
                    expected4_addresses[pos],
                    collection4.collection_services[pos].address
                );
                assert_eq!(
                    "urn:taxii.mitre.org:protocol:https:1.0",
                    collection4.collection_services[pos].protocol_binding
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.0",
                    collection4.collection_services[pos].message_bindings[0]
                );
                assert_eq!(
                    "urn:taxii.mitre.org:message:xml:1.1",
                    collection4.collection_services[pos].message_bindings[1]
                );
            }
            for pos in 0..expected_collection4_content_bindings.len() {
                assert_eq!(
                    expected_collection4_content_bindings[pos],
                    collection4.collection_services[4].content_bindings[pos]
                );
                assert_eq!(
                    expected_collection4_content_bindings[pos],
                    collection4.collection_services[5].content_bindings[pos]
                );
            }
        }
    }
}
