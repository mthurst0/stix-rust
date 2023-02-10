use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use tracing::info;

/*
MEDIA_TYPE_TAXII_ANY = "application/taxii+json"
MEDIA_TYPE_TAXII_V21 = "{media};version=2.1".format(media=MEDIA_TYPE_TAXII_ANY)


def validate_version_parameter_in_accept_header():
    """All endpoints need to check the Accept Header for the correct Media Type"""
    accept_header = request.headers.get("accept", "").replace(" ", "").split(",")
    found = False

    for item in accept_header:
        result = re.match(r"^application/taxii\+json(;version=(\d\.\d))?$", item)
        if result:
            if len(result.groups()) >= 1:
                version_str = result.group(2)
                if version_str != "2.1":  # The server only supports 2.1
                    raise ProcessingError("The server does not support version {}".format(version_str), 406)
            found = True
            break

    if found is False:
        raise ProcessingError("Media type in the Accept header is invalid or not found", 406) */

/*
Defines TAXII API - Server Information:
Server Discovery section (4.1) `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107526>`__

Returns:
    discovery: A Discovery Resource upon successful requests. Additional information
    `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107527>`__.
*/
#[get("/taxii2")]
async fn discovery(req: HttpRequest) -> HttpResponse {
    match req.headers().get("accept") {
        Some(v) => return HttpResponse::Ok().finish(),
        None => return HttpResponse::BadRequest().finish(),
    }
}

#[derive(Debug)]
pub struct ListenAddr {
    ip: String,
    port: u16,
}

impl ListenAddr {
    pub fn new(ip: &str, port: u16) -> ListenAddr {
        return ListenAddr {
            ip: String::from(ip),
            port,
        };
    }
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let addr = ListenAddr::new("127.0.0.1", 8080);
    info!("listening to: {:?}", addr);
    HttpServer::new(|| App::new().service(greet))
        .bind((addr.ip, addr.port))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use actix_web::{body::to_bytes, dev::Service, http, test, web, App, Error};

    use super::*;

    #[actix_web::test]
    async fn test_discovery() -> Result<(), Error> {
        let app = App::new().route("/taxii2", web::get().to(discovery));
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = app.call(req).await?;

        assert_eq!(resp.status(), http::StatusCode::OK);

        let response_body = resp.into_body();
        assert_eq!(to_bytes(response_body).await?, r##"Hello world!"##);

        Ok(())
    }
}
