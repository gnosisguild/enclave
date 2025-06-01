use anyhow::*;
use e3_entrypoint::nodes::stop;

pub async fn execute(id: &str) -> Result<()> {
    stop::execute(id).await?;
    Ok(())
}
