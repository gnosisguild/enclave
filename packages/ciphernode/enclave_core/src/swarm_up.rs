use anyhow::*;
use config::AppConfig;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn execute(
    config: &AppConfig,
    _detatch: bool, // TBI
    exclude: Vec<String>,
    verbose: u8,
    maybe_config_string: Option<String>,
) -> Result<()> {
    // if the webserver is running
    //   throw an error because swarm is already running

    // if I am in detatched mode
    //  start the webserver in a child process forwarding creds and return
    // else
    //  run the swarm_daemon process locally forwarding args

    Ok(())
}
