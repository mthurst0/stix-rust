use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    backend::{Backend, Filtering},
    errors::MyError,
    server::{CollectionConfig, ManifestRecord, Object},
};

pub struct FileBackend {
    root_dir: String,
}

#[derive(Deserialize, Serialize)]
struct FileCollection {
    config: CollectionConfig,
    object: Vec<Object>,
    manifest: Vec<ManifestRecord>,
}

impl Backend for FileBackend {
    fn get_manifests(
        &self,
        collection_id: &str,
        filtering: &Filtering,
    ) -> Result<Vec<ManifestRecord>, MyError> {
        let path = Path::new(self.root_dir.as_str()).join(format!("{}.json", collection_id));
        let collection = match std::fs::read_to_string(path) {
            Ok(v) => v,
            // TODO: not found error
            Err(err) => return Err(MyError(err.to_string())),
        };
        let collection = match serde_json::from_slice::<FileCollection>(collection.as_bytes()) {
            Ok(v) => v,
            Err(err) => return Err(MyError(err.to_string())),
        };
        let mut result = Vec::<ManifestRecord>::new();
        for rec in collection.manifest.iter().enumerate() {
            result.push(rec.1.clone());
        }
        Ok(result)
    }
}
