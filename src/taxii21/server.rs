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
pub struct APIRootConfig {
    title: String,
    description: Option<String>,
    versions: Vec<String>,
    max_content_length: u64,
}

impl APIRootConfig {
    pub fn new(
        title: &str,
        description: Option<&str>,
        versions: &Vec<String>,
        max_content_length: u64,
    ) -> APIRootConfig {
        return APIRootConfig {
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

#[derive(Clone)]
pub struct APIRoot {
    config: APIRootConfig,
    api_root_server_record_limit: Option<u32>,
    statii: HashMap<String, Status>,
    collections: Collections,
}

impl APIRoot {
    pub fn new(config: &APIRootConfig) -> APIRoot {
        return APIRoot {
            config: config.clone(),
            api_root_server_record_limit: None,
            statii: HashMap::<String, Status>::new(),
            collections: Collections::new(),
        };
    }
    pub fn add_status(&mut self, status: &Status) {
        self.statii.insert(status.id.clone(), status.clone());
    }
    pub fn add_collection(&mut self, collection: &CollectionConfig) {
        self.collections.add_collection(collection);
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct StatusDetails {
    id: String,
    version: String,
    message: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
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

impl Status {
    pub fn new(id: &str) -> Status {
        return Status {
            id: String::from(id),
            status: String::from(""),
            request_timestamp: None,
            total_count: 0,
            success_count: 0,
            successes: None,
            failure_count: 0,
            failures: None,
            pending_count: 0,
            pendings: None,
        };
    }
}

#[derive(Clone, Deserialize, Serialize)]
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

#[derive(Clone, Deserialize, Serialize)]
pub struct Collections {
    collections: Option<Vec<CollectionConfig>>,
}

impl Collections {
    pub fn new() -> Collections {
        return Collections { collections: None };
    }
    pub fn add_collection(&mut self, collection: &CollectionConfig) {
        match &mut self.collections {
            Some(collections) => collections.push(collection.clone()),
            None => {
                let mut new_collections = Vec::<CollectionConfig>::new();
                new_collections.push(collection.clone());
                self.collections = Some(new_collections);
            }
        }
    }
    pub fn get_collection(&self, id: &str) -> Option<&CollectionConfig> {
        match &self.collections {
            Some(collections) => {
                for (pos, collection) in collections.iter().enumerate() {
                    if collection.id == id {
                        return Some(&collections[pos]);
                    }
                }
                None
            }
            None => None,
        }
    }
}

#[derive(Clone)]
pub struct Collection {
    pub config: CollectionConfig,
    pub manifests: Vec<ManifestRecord>,
}

impl Collection {
    pub fn new(id: &str, title: &str) -> Collection {
        return Collection {
            config: CollectionConfig::new(id, title),
            manifests: Vec::<ManifestRecord>::new(),
        };
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CollectionConfig {
    id: String,
    title: String,
    description: Option<String>,
    alias: Option<String>,
    can_read: bool,
    can_write: bool,
    media_types: Option<Vec<String>>,
}

impl CollectionConfig {
    pub fn new(id: &str, title: &str) -> CollectionConfig {
        return CollectionConfig {
            id: String::from(id),
            title: String::from(title),
            description: None,
            alias: None,
            can_read: false,
            can_write: false,
            media_types: None,
        };
    }
}

#[derive(Clone, Serialize)]
pub struct Manifest {
    more: Option<bool>,
    objects: Option<Vec<ManifestRecord>>,
}

#[derive(Clone, Deserialize, Serialize)]
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
    pub default_server_record_limit: u32,
    pub api_roots: HashMap<String, APIRoot>,
}

const DEFAULT_SERVER_LIMIT: u32 = 100;

impl AppState {
    pub fn new_empty() -> AppState {
        return AppState {
            server: Discovery::new_empty(),
            default_server_record_limit: DEFAULT_SERVER_LIMIT,
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
    pub fn add_status(&mut self, api_root: &str, status: &Status) -> Result<(), MyError> {
        let api_root = match self.api_roots.get_mut(api_root) {
            Some(v) => v,
            // TODO: errors -- e.g. "Not Found"
            // TODO: see actix_web examples
            None => return Err(MyError(format!("could not find api_root={}", api_root))),
        };
        api_root.add_status(status);
        Ok(())
    }
    pub fn get_status(&self, api_root: &str, status_id: &str) -> Option<Status> {
        match self.api_roots.get(api_root) {
            Some(api_root) => match api_root.statii.get(status_id) {
                Some(status) => return Some(status.clone()),
                None => return None,
            },
            None => return None,
        };
    }
    pub fn add_collection(
        &mut self,
        api_root: &str,
        collection: &CollectionConfig,
    ) -> Result<(), MyError> {
        let api_root = match self.api_roots.get_mut(api_root) {
            Some(v) => v,
            // TODO: errors -- e.g. "Not Found"
            // TODO: see actix_web examples
            None => return Err(MyError(format!("could not find api_root={}", api_root))),
        };
        api_root.add_collection(collection);
        Ok(())
    }
    pub fn get_collections(&self, api_root: &str) -> Option<&Collections> {
        match self.api_roots.get(api_root) {
            Some(api_root) => return Some(&api_root.collections),
            None => return None,
        };
    }
}

const CONTENT_TYPE_TAXII2: &'static str = "application/taxii+json;version=2.1";

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
    let config = match app_state.api_roots.get(&path.api_root) {
        Some(v) => v.config.clone(),
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", CONTENT_TYPE_TAXII2))
        .json(web::Json(config)))
}

#[derive(Deserialize)]
struct APIRootStatusPath {
    api_root: String,
    status_id: String,
}

async fn handle_api_root_status(
    wrapper: web::Data<AppStateWrapper>,
    path: web::Path<APIRootStatusPath>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let app_state = wrapper.app_state.lock().unwrap();
    let status = match app_state.get_status(path.api_root.as_str(), path.status_id.as_str()) {
        Some(v) => v,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", CONTENT_TYPE_TAXII2))
        .json(web::Json(status)))
}

async fn handle_api_root_collections(
    wrapper: web::Data<AppStateWrapper>,
    path: web::Path<APIRootPath>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let app_state = wrapper.app_state.lock().unwrap();
    let collections = match app_state.get_collections(path.api_root.as_str()) {
        Some(v) => v,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", CONTENT_TYPE_TAXII2))
        .json(web::Json(collections)))
}

#[derive(Deserialize)]
struct APIRootCollectionPath {
    api_root: String,
    collection_id: String,
}

async fn handle_api_root_collection(
    wrapper: web::Data<AppStateWrapper>,
    path: web::Path<APIRootCollectionPath>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let app_state = wrapper.app_state.lock().unwrap();
    let collections = match app_state.get_collections(path.api_root.as_str()) {
        Some(v) => v,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    let collection = match collections.get_collection(path.collection_id.as_str()) {
        Some(v) => v,
        None => return Ok(HttpResponse::NotFound().finish()),
    };
    Ok(HttpResponse::Ok()
        .append_header(("Content-Type", CONTENT_TYPE_TAXII2))
        .json(web::Json(collection)))
}

async fn handle_api_root_collection_manifests(
    wrapper: web::Data<AppStateWrapper>,
    path: web::Path<APIRootCollectionPath>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    // TODO
    Ok(HttpResponse::NotFound().finish())
    // let app_state = wrapper.app_state.lock().unwrap();
    // let collections = match app_state.get_collections(path.api_root.as_str()) {
    //     Some(v) => v,
    //     None => return Ok(HttpResponse::NotFound().finish()),
    // };
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
        .service(web::resource("/{api_root}/").route(web::get().to(handle_api_root)))
        .service(
            web::resource("/{api_root}/status/{status_id}/")
                .route(web::get().to(handle_api_root_status)),
        )
        .service(
            web::resource("/{api_root}/collections/")
                .route(web::get().to(handle_api_root_collections)),
        )
        .service(
            web::resource("/{api_root}/collections/{collection_id}/")
                .route(web::get().to(handle_api_root_collection)),
        );
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
                APIRoot::new(&APIRootConfig::new(
                    "api-root-title",
                    Some("api-root-description"),
                    &versions,
                    1000,
                )),
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
        let api_root: APIRootConfig =
            match serde_json::from_slice::<APIRootConfig>(response_body.as_ref()) {
                Ok(v) => v,
                Err(err) => panic!("err={}", err),
            };
        assert_eq!("api-root-title", api_root.title);
        assert_eq!("api-root-description", api_root.description.unwrap());
        assert_eq!("api-root-version", api_root.versions[0]);
        assert_eq!(1000, api_root.max_content_length);

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

    #[actix_web::test]
    async fn test_handle_api_root_status() -> Result<(), Error> {
        let app_state = Arc::new(Mutex::new(AppState::new_empty()));
        let app = new_app(app_state.clone());
        let app = test::init_service(app).await;
        let mut versions = Vec::<String>::new();
        versions.push(String::from("api-root-version"));
        {
            let mut app_state = app_state.lock().unwrap();
            app_state.api_roots.insert(
                String::from("api_root1"),
                APIRoot::new(&APIRootConfig::new(
                    "api-root-title",
                    Some("api-root-description"),
                    &versions,
                    1000,
                )),
            );
        }
        let req = test::TestRequest::get()
            .uri("/api_root1/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);

        let req = test::TestRequest::get()
            .uri("/api_root1/status/test-status-id")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
        {
            let mut app_state = app_state.lock().unwrap();
            let mut status = Status::new("test-status-id");
            status.status = String::from("SUCCESS");
            app_state.add_status("api_root1", &status).unwrap();
        }

        let req = test::TestRequest::get()
            .uri("/api_root1/status/test-status-id/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let response_body = to_bytes(resp.into_body()).await?;
        assert!(response_body.len() > 0);
        let status: Status = match serde_json::from_slice::<Status>(response_body.as_ref()) {
            Ok(v) => v,
            Err(err) => panic!("err={}", err),
        };
        assert_eq!("test-status-id", status.id);
        assert_eq!("SUCCESS", status.status);

        Ok(())
    }

    #[actix_web::test]
    async fn test_handle_api_root_collections() -> Result<(), Error> {
        let app_state = Arc::new(Mutex::new(AppState::new_empty()));
        let app = new_app(app_state.clone());
        let app = test::init_service(app).await;
        let mut versions = Vec::<String>::new();
        versions.push(String::from("api-root-version"));
        {
            let mut app_state = app_state.lock().unwrap();
            app_state.api_roots.insert(
                String::from("api_root1"),
                APIRoot::new(&APIRootConfig::new(
                    "api-root-title",
                    Some("api-root-description"),
                    &versions,
                    1000,
                )),
            );
        }
        let req = test::TestRequest::get()
            .uri("/api_root1/collections/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let response_body = to_bytes(resp.into_body()).await?;
        assert!(response_body.len() > 0);
        let collections: Collections =
            match serde_json::from_slice::<Collections>(response_body.as_ref()) {
                Ok(v) => v,
                Err(err) => panic!("err={}", err),
            };
        assert!(collections.collections.is_none());

        {
            let mut app_state = app_state.lock().unwrap();
            let collection = CollectionConfig::new("collection-id", "collection-title");
            match app_state.add_collection("api_root1", &collection) {
                Ok(_) => (),
                Err(err) => panic!("err={}", err),
            }
        }
        let req = test::TestRequest::get()
            .uri("/api_root1/collections/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let response_body = to_bytes(resp.into_body()).await?;
        assert!(response_body.len() > 0);
        let collections: Collections =
            match serde_json::from_slice::<Collections>(response_body.as_ref()) {
                Ok(v) => v,
                Err(err) => panic!("err={}", err),
            };
        match collections.collections {
            Some(collections) => {
                assert_eq!(1, collections.len());
                assert_eq!("collection-id", collections[0].id);
                assert_eq!("collection-title", collections[0].title);
            }
            None => panic!("expected one collection"),
        }

        Ok(())
    }

    #[actix_web::test]
    async fn test_handle_api_root_collection() -> Result<(), Error> {
        let app_state = Arc::new(Mutex::new(AppState::new_empty()));
        let app = new_app(app_state.clone());
        let app = test::init_service(app).await;
        let mut versions = Vec::<String>::new();
        versions.push(String::from("api-root-version"));
        {
            let mut app_state = app_state.lock().unwrap();
            app_state.api_roots.insert(
                String::from("api_root1"),
                APIRoot::new(&APIRootConfig::new(
                    "api-root-title",
                    Some("api-root-description"),
                    &versions,
                    1000,
                )),
            );
        }
        let req = test::TestRequest::get()
            .uri("/api_root1/collections/test-collection-id/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);

        {
            let mut app_state = app_state.lock().unwrap();
            let collection = CollectionConfig::new("test-collection-id", "test-collection-title");
            match app_state.add_collection("api_root1", &collection) {
                Ok(_) => (),
                Err(err) => panic!("err={}", err),
            }
        }
        let req = test::TestRequest::get()
            .uri("/api_root1/collections/test-collection-id/")
            .append_header(("Accept", "application/taxii+json;version=2.1"))
            .to_request();
        let resp = app.call(req).await?;
        assert_eq!(resp.status(), http::StatusCode::OK);
        let response_body = to_bytes(resp.into_body()).await?;
        assert!(response_body.len() > 0);
        let collection: CollectionConfig =
            match serde_json::from_slice::<CollectionConfig>(response_body.as_ref()) {
                Ok(v) => v,
                Err(err) => panic!("err={}", err),
            };
        assert_eq!("test-collection-id", collection.id);
        assert_eq!("test-collection-title", collection.title);

        Ok(())
    }
}
