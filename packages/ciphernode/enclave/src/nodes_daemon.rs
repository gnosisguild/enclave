use anyhow::*;
use config::AppConfig;
use enclave_core::nodes::daemon;

pub async fn execute(
    config: &AppConfig,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
    otel: Option<String>,
) -> Result<()> {
    daemon::execute(config, exclude, verbose, config_string, otel).await
}
