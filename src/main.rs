#![allow(dead_code)]
#![allow(unused_variables)]

// TODO: remove the above once we get more stable

pub mod taxii;

fn main() {
    let discovery_request_url = "https://test.taxiistand.com/read-write/services/discovery";
    let collection_information_request_url =
        "https://test.taxiistand.com/read-write/services/collection-management";
    let username = "guest";
    let password = "guest";
    let ver = taxii::version::Version::V11;
    // taxii::version::discovery_request(discovery_request_url, username, password, ver);
    taxii::version::collection_information_request(
        collection_information_request_url,
        username,
        password,
        ver,
    );
}
