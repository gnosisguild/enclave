use anyhow::*;
use e3_config::AppConfig;
use e3_entrypoint::nodes::up;

pub async fn execute(
    config: &AppConfig,
    detach: bool,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
    otel: Option<String>,
) -> Result<()> {
    up::execute(config, detach, exclude, verbose, config_string, otel).await
}
