use std::path::PathBuf;

use anyhow::Result;

pub async fn execute(location: Option<PathBuf>, template: Option<String>) -> Result<()> {
    e3_init::execute(location, template).await
}
