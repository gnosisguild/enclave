use anyhow::*;
use config::AppConfig;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn execute(config: &AppConfig, detatch: bool) -> Result<()> {
    println!("Hello world");
    Ok(())
}
