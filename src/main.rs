#![allow(dead_code)]
#![allow(unused_variables)]

// TODO: remove the above once we get more stable
// TODO: excessive String cloning in message parsing

pub mod taxii;

fn main() {
    let username = "guest";
    let password = "guest";
    let ver = taxii::version::Version::V11;
    // let discovery_request_url = "https://test.taxiistand.com/read-write/services/discovery";
    // taxii::version::discovery_request(discovery_request_url, username, password, ver);

    //let collection_information_request_url =
    //  "https://test.taxiistand.com/read-write/services/collection-management";
    // taxii::version::collection_information_request(
    //     collection_information_request_url,
    //     username,
    //     password,
    //     ver,
    // );

    let subscribe_request_url =
        "https://test.taxiistand.com/read-write/services/collection-management";
    taxii::subscriptions::subscribe_request(
        subscribe_request_url,
        username,
        password,
        ver,
        "stix-data",
    );
}
