use anyhow::Result;

pub async fn execute() -> Result<()> {
    enclave_init::execute().await
}
