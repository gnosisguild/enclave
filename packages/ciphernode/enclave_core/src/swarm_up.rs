use crate::{helpers::swarm_client, swarm_daemon};
use anyhow::*;
use config::AppConfig;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn execute(
    config: &AppConfig,
    detatch: bool, // TBI
    exclude: Vec<String>,
    verbose: u8,
    maybe_config_string: Option<String>,
) -> Result<()> {
    if swarm_client::is_ready().await? {
        bail!("Swarm is already running!");
    }

    if detatch {
        swarm_client::start_daemon(verbose, &maybe_config_string, &exclude).await?;
        return Ok(());
    }

    //  run the swarm_daemon process locally forwarding args
    swarm_daemon::execute(config, exclude, verbose, maybe_config_string).await?;

    Ok(())
}
