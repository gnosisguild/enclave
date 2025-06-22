use std::path::PathBuf;

use anyhow::Result;

pub async fn execute(
    location: Option<PathBuf>,
    template: Option<String>,
    skip_cleanup: bool,
) -> Result<()> {
    e3_init::execute(location, template, skip_cleanup).await
}
