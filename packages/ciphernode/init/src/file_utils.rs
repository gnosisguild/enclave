use std::path::Path;

use anyhow::{bail, Result};
use tokio::fs;

pub async fn ensure_empty_folder<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        fs::create_dir(path).await?;
    }

    if !path.is_dir() {
        bail!("Path '{}' is not a directory", path.display());
    }

    let mut entries = std::fs::read_dir(path)
        .map_err(|e| anyhow::anyhow!("Failed to read directory '{}': {}", path.display(), e))?;

    if entries.next().is_some() {
        bail!("Directory '{}' is not empty", path.display());
    }

    Ok(())
}

pub async fn delete_path<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if path.is_dir() {
        fs::remove_dir_all(path).await?;
    } else {
        fs::remove_file(path).await?;
    }

    Ok(())
}
