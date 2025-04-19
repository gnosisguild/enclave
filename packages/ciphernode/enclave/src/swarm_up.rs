use anyhow::*;
use config::AppConfig;
use enclave_core::{swarm_daemon, swarm_up};

pub async fn execute(
    config: &AppConfig,
    detatch: bool,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
) -> Result<()> {
    swarm_daemon::execute(config, detatch, exclude, verbose, config_string).await
}
