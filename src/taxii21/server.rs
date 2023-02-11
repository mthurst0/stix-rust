use std::path::Path;

use crate::taxii21::middleware;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use tracing::info;

use super::errors::MyError;

#[derive(Clone, Serialize)]
pub struct Server {
    title: String,
    description: Option<String>,
    contact: Option<String>,
    default: String,
    api_roots: Vec<String>,
}

impl Server {
    pub fn new_empty() -> Server {
        return Server {
            title: String::new(),
            description: None,
            contact: None,
            default: String::new(),
            api_roots: Vec::<String>::new(),
        };
    }
}

#[derive(Deserialize, Serialize)]
pub struct Taxii2ServerConfig {
    title: String,
    description: Option<String>,
    contact: Option<String>,
    default: String,
    api_roots: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct AppConfig {
    taxii2_server: Taxii2ServerConfig,
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
    pub fn load_toml(path: &Path) -> Result<AppState, MyError> {
        let cfg = match std::fs::read_to_string(path) {
            Ok(cfg) => cfg,
            Err(err) => return Err(MyError(err.to_string())),
        };
        let cfg: AppConfig = match toml::from_str(cfg.as_str()) {
            Ok(cfg) => cfg,
            Err(err) => return Err(MyError(err.to_string())),
        };
        let mut app_state = AppState::new_empty();
        app_state.server.title = cfg.taxii2_server.title;
        app_state.server.description = cfg.taxii2_server.description;
        app_state.server.contact = cfg.taxii2_server.contact;
        app_state.server.default = cfg.taxii2_server.default;
        app_state.server.api_roots = cfg.taxii2_server.api_roots;
        Ok(app_state)
    }
}

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
    let path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(path.as_str()).join("test/sample-server.toml");
    let app_state = match AppState::load_toml(path.as_path()) {
        Ok(app_state) => app_state,
        Err(err) => panic!("err={}", err),
    };
    let addr = ListenAddr::new("127.0.0.1", 8080);
    info!("listening: {}:{}", addr.ip, addr.port);
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
