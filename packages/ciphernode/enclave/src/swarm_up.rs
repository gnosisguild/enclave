use anyhow::*;
use config::AppConfig;
use enclave_core::swarm_up;

pub async fn execute(
    config: &AppConfig,
    detach: bool,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
) -> Result<()> {
    swarm_up::execute(config, detach, exclude, verbose, config_string).await
}
