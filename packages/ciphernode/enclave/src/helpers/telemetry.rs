use anyhow::Result;
use config::AppConfig;
use tracing::Level;

pub fn setup_tracing(_config: &AppConfig, log_level: Level) -> Result<()> {
    tracing_subscriber::fmt().with_max_level(log_level).init();
    Ok(())
}
