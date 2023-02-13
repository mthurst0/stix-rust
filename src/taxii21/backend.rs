use super::{errors::MyError, server::ManifestRecord};

#[derive(Clone)]
pub enum MatchField {
    Id,
    SpecVersion,
    Type,
    Version,
}

pub struct Match {
    field: MatchField,
    values: Vec<String>,
}

pub struct Filtering {
    added_after: chrono::DateTime<chrono::Utc>,
    limit: u32,
    next: String,
    matches: Vec<Match>,
}

pub trait Backend {
    fn get_manifests(
        &self,
        collection_id: &str,
        filtering: &Filtering,
    ) -> Result<Vec<ManifestRecord>, MyError>;
}
