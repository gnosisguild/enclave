use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait ProgramSupportApi {
    async fn compile(&self) -> Result<()>;
    async fn start(&self) -> Result<()>;
}
