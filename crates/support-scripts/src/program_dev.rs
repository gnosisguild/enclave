use crate::traits::ProgramSupportApi;
use anyhow::Result;
use async_trait::async_trait;
use e3_config::ProgramConfig;

pub struct ProgramSupportDev(pub ProgramConfig);

#[async_trait]
impl ProgramSupportApi for ProgramSupportDev {
    async fn compile(&self) -> Result<()> {
        println!("compile");
        Ok(())
    }
    async fn start(&self) -> Result<()> {
        println!("start");
        Ok(())
    }
}
