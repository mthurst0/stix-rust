use std::{self, io::Write};

use chrono::Utc;

pub fn write_cache_file_with_filestamp(stub_name: &str, data: &str) -> std::io::Result<()> {
    let path = std::env::var("HOME").unwrap();
    let filestamp = Utc::now()
        .format(format!("%Y-%m-%d-%H-%M-%S-{}", stub_name).as_str())
        .to_string();
    let path = std::path::Path::new(path.as_str())
        .join(".rkcache")
        .join(filestamp);
    let mut file = std::fs::File::create(path)?;
    file.write_all(data.as_bytes())?;
    Ok(())
}
