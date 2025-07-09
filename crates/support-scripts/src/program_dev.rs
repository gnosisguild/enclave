use std::env;

use crate::{
    traits::ProgramSupportApi,
    utils::{ensure_script_exists, run_bash_script},
};
use anyhow::Result;
use async_trait::async_trait;
use e3_config::ProgramConfig;

pub struct ProgramSupportDev(pub ProgramConfig);

#[async_trait]
impl ProgramSupportApi for ProgramSupportDev {
    async fn compile(&self) -> Result<()> {
        let cwd = env::current_dir()?;
        let script = cwd.join(".enclave/support/dev/compile");
        ensure_script_exists(&script).await?;
        run_bash_script(&cwd, &script, &[]).await?;
        Ok(())
    }
    async fn start(&self) -> Result<()> {
        let cwd = env::current_dir()?;
        let script = cwd.join(".enclave/support/dev/start");
        ensure_script_exists(&script).await?;
        run_bash_script(&cwd, &script, &[]).await?;
        Ok(())
    }
}
