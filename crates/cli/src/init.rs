use anyhow::Result;

pub async fn execute() -> Result<()> {
    e3_init::execute().await
}
