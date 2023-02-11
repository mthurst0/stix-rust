use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use tracing::info;

/*
Defines TAXII API - Server Information:
Server Discovery section (4.1) `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107526>`__

Returns:
    discovery: A Discovery Resource upon successful requests. Additional information
    `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107527>`__.
*/
async fn discovery(req: HttpRequest) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().finish())
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
    HttpServer::new(|| App::new().service(web::resource("/taxii2").route(web::get().to(discovery))))
        .bind((addr.ip, addr.port))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use actix_web::{body::to_bytes, dev::Service, http, test, web, App, Error};

    use crate::taxii21::middleware;

    use super::*;

    #[actix_web::test]
    async fn test_discovery() -> Result<(), Error> {
        let app = App::new()
            .wrap(middleware::CheckAcceptHeader)
            .route("/taxii2", web::get().to(discovery));
        let app = test::init_service(app).await;

        let req = test::TestRequest::get().uri("/taxii2").to_request();
        let resp = app.call(req).await?;

        assert_eq!(resp.status(), http::StatusCode::NOT_ACCEPTABLE);

        let response_body = resp.into_body();
        assert_eq!(to_bytes(response_body).await?.len(), 0);

        let req = test::TestRequest::get()
            .uri("/taxii2")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();

        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);

        let req = test::TestRequest::get()
            .uri("/taxii2")
            .append_header(("Accept", "application/taxii+json;version=1.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_ACCEPTABLE);

        Ok(())
    }
}
