use anyhow::*;
use config::AppConfig;
use enclave_core::nodes::up;

pub async fn execute(
    config: &AppConfig,
    detach: bool,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
) -> Result<()> {
    up::execute(config, detach, exclude, verbose, config_string).await
}
