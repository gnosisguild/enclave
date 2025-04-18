use anyhow::*;
use config::AppConfig;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn execute(config: &AppConfig, detatch: bool) -> Result<()> {
    println!("Hello world");
    // get each node name to take part in the swarm
    // get a list of all multiaddrs for each node within the swarm
    // get the local values of -v and --config as strings
    // get each node's peer list based on the other nodes in the swarm
    // assemble process command `enclave start` passing on (-v) (--config) (--peer) (--otel)
    // run the command forward the output to the current terminal adding a coloured prefix and pass on SIGTERM
    Ok(())
}
