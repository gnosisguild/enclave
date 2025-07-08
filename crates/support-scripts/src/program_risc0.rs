use std::env;

use crate::{ensure_script_exists, run_bash_script, traits::ProgramSupportApi};
use anyhow::Result;
use async_trait::async_trait;
use e3_config::ProgramConfig;

pub struct ProgramSupportRisc0(pub ProgramConfig);

#[async_trait]
impl ProgramSupportApi for ProgramSupportRisc0 {
    /// Run the docker container compile script
    async fn compile(&self) -> Result<()> {
        let cwd = env::current_dir()?;
        let script = cwd.join(".enclave/support/ctl/compile");
        ensure_script_exists(&script).await?;
        run_bash_script(&cwd, &script, &[]).await?;
        Ok(())
    }

    /// Run the docker container start script
    async fn start(&self) -> Result<()> {
        let cwd = env::current_dir()?;
        let script = cwd.join(".enclave/support/ctl/start");
        ensure_script_exists(&script).await?;

        let risc0_config = self.0.risc0();
        let risc0_dev_mode_str = risc0_config.risc0_dev_mode.to_string();

        let mut args = vec!["--risc0-dev-mode", risc0_dev_mode_str.as_str()];

        if let (Some(api_key), Some(api_url)) =
            (&risc0_config.bonsai_api_key, &risc0_config.bonsai_api_url)
        {
            args.extend(["--api-key", api_key.as_str(), "--api-url", api_url.as_str()]);
        }

        run_bash_script(&cwd, &script, &args).await?;
        Ok(())
    }
}
