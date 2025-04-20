use anyhow::*;
use config::AppConfig;
use enclave_core::swarm_daemon;

pub async fn execute(
    config: &AppConfig,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
) -> Result<()> {
    swarm_daemon::execute(config, exclude, verbose, config_string).await
}
