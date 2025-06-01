use std::path::PathBuf;

use anyhow::Result;
use serde_json::Value;
use tokio::fs;

pub async fn get_version_from_package_json(file_path: &PathBuf) -> Result<String> {
    let content = fs::read_to_string(file_path).await?;
    let json: Value = serde_json::from_str(&content)?;

    json["version"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("version field not found or not a string"))
}
