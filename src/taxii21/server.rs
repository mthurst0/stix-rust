use crate::taxii21::middleware;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use serde::Serialize;
use tracing::info;

#[derive(Clone, Serialize)]
pub struct Server {
    title: String,
    description: String,
    contact: String,
    default: String,
    api_roots: Vec<String>,
}

impl Server {
    pub fn new_empty() -> Server {
        return Server {
            title: String::new(),
            description: String::new(),
            contact: String::new(),
            default: String::new(),
            api_roots: Vec::<String>::new(),
        };
    }
}

#[derive(Clone)]
struct AppState {
    server: Server,
}

impl AppState {
    pub fn new_empty() -> AppState {
        return AppState {
            server: Server::new_empty(),
        };
    }
}

/*
"title": "Some TAXII Server",
"description": "This TAXII Server contains a listing of",
"contact": "string containing contact information",
"default": "http://localhost:5000/trustgroup1/",
"api_roots": [
    "http://localhost:5000/api1/",
    "http://localhost:5000/api2/",
    "http://localhost:5000/trustgroup1/"
]
*/

/*
Defines TAXII API - Server Information:
Server Discovery section (4.1) `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107526>`__

Returns:
    discovery: A Discovery Resource upon successful requests. Additional information
    `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107527>`__.
*/
async fn discovery(app_data: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let server = &app_data.server;
    Ok(HttpResponse::Ok().json(web::Json(server)))
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
    let app_state = AppState::new_empty();
    let addr = ListenAddr::new("127.0.0.1", 8080);
    info!("listening to: {:?}", addr);
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(middleware::CheckAcceptHeader)
            .service(web::resource("/taxii2").route(web::get().to(discovery)))
    })
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
        let app_state = AppState::new_empty();
        let app = App::new()
            .app_data(web::Data::new(app_state.clone()))
            .wrap(middleware::CheckAcceptHeader)
            .service(web::resource("/taxii2").route(web::get().to(discovery)));
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
