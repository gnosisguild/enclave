use std::path::PathBuf;

use anyhow::Result;

pub async fn execute(location: Option<PathBuf>) -> Result<()> {
    e3_init::execute(location).await
}
