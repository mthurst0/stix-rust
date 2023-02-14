use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::info;

use super::{
    backend::{Backend, Filtering},
    errors::MyError,
    server::{CollectionConfig, ManifestRecord, Object},
};

pub struct FileBackend {
    root_dir: String,
}

impl FileBackend {
    pub fn new(root_dir: &str) -> FileBackend {
        return FileBackend {
            root_dir: String::from(root_dir),
        };
    }
}

#[derive(Deserialize, Serialize)]
struct FileCollection {
    config: CollectionConfig,
    objects: Vec<Object>,
    manifest: Vec<ManifestRecord>,
}

impl Backend for FileBackend {
    fn get_manifests(
        &self,
        collection_id: &str,
        filtering: &Filtering,
    ) -> Result<Vec<ManifestRecord>, MyError> {
        let path =
            Path::new(self.root_dir.as_str()).join(format!("collection-{}.json", collection_id));
        let collection = match std::fs::read_to_string(path) {
            Ok(v) => v,
            // TODO: not found error
            Err(err) => return Err(MyError(err.to_string())),
        };
        let collection = match serde_json::from_slice::<FileCollection>(collection.as_bytes()) {
            Ok(v) => v,
            Err(err) => {
                info!("err-in-json={}", err);
                let msg = err.to_string();
                return Err(MyError(err.to_string()));
            }
        };
        let mut result = Vec::<ManifestRecord>::new();
        collection
            .manifest
            .iter()
            .for_each(|rec| result.push(rec.clone()));
        Ok(result)
    }
}
