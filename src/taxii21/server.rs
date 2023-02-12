use std::{collections::HashMap, path::Path, sync::Arc};

use crate::taxii21::middleware;
use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, App, Error, HttpRequest, HttpResponse, HttpServer,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tracing::info;

use super::errors::MyError;

#[derive(Clone, Serialize)]
pub struct Discovery {
    title: String,
    description: Option<String>,
    contact: Option<String>,
    default: Option<String>,
    api_roots: Option<Vec<String>>,
}

impl Discovery {
    pub fn new_empty() -> Discovery {
        return Discovery {
            title: String::new(),
            description: None,
            contact: None,
            default: None,
            api_roots: None,
        };
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct APIRoot {
    title: String,
    description: Option<String>,
    versions: Vec<String>,
    max_content_length: u64,
}

impl APIRoot {
    pub fn new(
        title: &str,
        description: Option<&str>,
        versions: &Vec<String>,
        max_content_length: u64,
    ) -> APIRoot {
        return APIRoot {
            title: String::from(title),
            description: match description {
                Some(v) => Some(String::from(v)),
                None => None,
            },
            versions: versions.clone(),
            max_content_length,
        };
    }
}

#[derive(Clone, Serialize)]
pub struct StatusDetails {
    id: String,
    version: String,
    message: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct Status {
    id: String,
    status: String, // TODO: StatusEnum
    request_timestamp: Option<DateTime<Utc>>,
    total_count: u32,
    success_count: u32,
    successes: Option<Vec<StatusDetails>>,
    failure_count: u32,
    failures: Option<Vec<StatusDetails>>,
    pending_count: u32,
    pendings: Option<Vec<StatusDetails>>,
}

#[derive(Clone, Serialize)]
pub struct Object {
    created: Option<DateTime<Utc>>,
    description: String,
    id: String,
    indicator_types: Vec<String>,
    is_family: bool,
    malware_types: Vec<String>,
    modified: Option<DateTime<Utc>>,
    name: String,
    pattern: String,
    pattern_type: String, // TODO: enum
    spec_version: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    typ: String,
    valid_from: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize)]
pub struct Collections {
    collections: Option<Vec<Collection>>,
}

#[derive(Clone, Serialize)]
pub struct Collection {
    id: String,
    title: String,
    description: Option<String>,
    alias: Option<String>,
    can_read: bool,
    can_write: bool,
    media_types: Option<Vec<String>>,
}

#[derive(Clone, Serialize)]
pub struct Manifest {
    more: Option<bool>,
    objects: Option<Vec<ManifestRecord>>,
}

#[derive(Clone, Serialize)]
pub struct ManifestRecord {
    id: String,
    date_added: chrono::DateTime<Utc>,
    version: String,
    media_type: Option<String>,
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
struct AppStateWrapper {
    app_state: Arc<Mutex<AppState>>,
}

#[derive(Clone)]
struct AppState {
    pub server: Discovery,
    pub api_roots: HashMap<String, APIRoot>,
}

impl AppState {
    pub fn new_empty() -> AppState {
        return AppState {
            server: Discovery::new_empty(),
            api_roots: HashMap::<String, APIRoot>::new(),
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
        app_state.server.default = Some(cfg.taxii2_server.default);
        app_state.server.api_roots = Some(cfg.taxii2_server.api_roots);
        Ok(app_state)
    }
}

const CONTENT_TYPE_TAXII2: &'static str = "application/taxii+json;version=2.1";

/*
Defines TAXII API - Server Information:
Server Discovery section (4.1) `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107526>`__

Returns:
    discovery: A Discovery Resource upon successful requests. Additional information
    `here <https://docs.oasis-open.org/cti/taxii/v2.1/cs01/taxii-v2.1-cs01.html#_Toc31107527>`__.
*/
async fn handle_discovery(
    wrapper: web::Data<AppStateWrapper>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let app_state = wrapper.app_state.lock().unwrap();
    let server = &app_state.server;
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", CONTENT_TYPE_TAXII2))
        .json(web::Json(server)))
}

#[derive(Deserialize)]
struct APIRootPath {
    api_root: String,
}

async fn handle_api_root(
    wrapper: web::Data<AppStateWrapper>,
    path: web::Path<APIRootPath>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let app_state = wrapper.app_state.lock().unwrap();
    let api_root = match app_state.api_roots.get(&path.api_root) {
        Some(v) => v,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", CONTENT_TYPE_TAXII2))
        .json(web::Json(api_root)))
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

fn new_app(
    app_state: Arc<Mutex<AppState>>,
) -> actix_web::App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<EitherBody<BoxBody>>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let wrapper = AppStateWrapper { app_state };
    return App::new()
        .app_data(web::Data::new(wrapper.clone()))
        .wrap(middleware::CheckAcceptHeader)
        .service(web::resource("/taxii2").route(web::get().to(handle_discovery)))
        .service(web::resource("/{api_root}/").route(web::get().to(handle_api_root)));
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let path = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = std::path::Path::new(path.as_str()).join("test/sample-server.toml");
    let app_state = match AppState::load_toml(path.as_path()) {
        Ok(app_state) => app_state,
        Err(err) => panic!("err={}", err),
    };
    let app_state = Arc::new(Mutex::new(app_state));
    let addr = ListenAddr::new("127.0.0.1", 8080);
    info!("listening: {}:{}", addr.ip, addr.port);
    HttpServer::new(move || new_app(app_state.clone()))
        .bind((addr.ip, addr.port))?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::{body::to_bytes, dev::Service, http, test, Error};

    use super::*;

    #[actix_web::test]
    async fn test_discovery() -> Result<(), Error> {
        let app_state = Arc::new(Mutex::new(AppState::new_empty()));
        let app = new_app(app_state.clone());
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
        assert_eq!(
            "application/taxii+json;version=2.1",
            resp.headers().get("Content-Type").unwrap()
        );

        let req = test::TestRequest::get()
            .uri("/taxii2")
            .append_header(("Accept", "application/taxii+json;version=1.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_ACCEPTABLE);

        Ok(())
    }

    #[actix_web::test]
    async fn test_handle_api_root_errors() -> Result<(), Error> {
        let app_state = Arc::new(Mutex::new(AppState::new_empty()));
        let app = new_app(app_state.clone());
        let app = test::init_service(app).await;

        let req = test::TestRequest::get()
            .uri("/not-found/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
        let response_body = resp.into_body();
        assert_eq!(to_bytes(response_body).await?.len(), 0);

        // TODO: test what happens with the OASIS implementation when accessing this URL
        let req = test::TestRequest::get()
            .uri("/taxii2/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
        let response_body = resp.into_body();
        assert_eq!(to_bytes(response_body).await?.len(), 0);

        let mut versions = Vec::<String>::new();
        versions.push(String::from("api-root-version"));
        {
            let mut app_state = app_state.lock().unwrap();
            app_state.api_roots.insert(
                String::from("api_root1"),
                APIRoot::new(
                    "api-root-title",
                    Some("api-root-description"),
                    &versions,
                    1000,
                ),
            );
        }

        let req = test::TestRequest::get()
            .uri("/api_root1/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let response_body = resp.into_body();
        let response_body = to_bytes(response_body).await?;
        assert!(response_body.len() > 0);
        let api_root: APIRoot = match serde_json::from_slice::<APIRoot>(response_body.as_ref()) {
            Ok(v) => v,
            Err(err) => panic!("err={}", err),
        };

        {
            let mut app_state = app_state.lock().unwrap();
            app_state.api_roots.remove(&String::from("api_root1"));
        }

        let req = test::TestRequest::get()
            .uri("/api_root1/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);

        Ok(())
    }
}
