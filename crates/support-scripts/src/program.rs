use std::env;

use anyhow::Result;
use async_trait::async_trait;
use e3_config::ProgramConfig;

use crate::{ensure_script_exists, run_bash_script};

pub enum ProgramSupport {
    Dev(ProgramSupportDev),
    Risc0(ProgramSupportRisc0),
}

impl ProgramSupport {
    pub fn new(config: ProgramConfig, mode: bool) -> ProgramSupport {
        if mode {
            ProgramSupport::Dev(ProgramSupportDev(config))
        } else {
            ProgramSupport::Risc0(ProgramSupportRisc0(config))
        }
    }
}

#[async_trait]
impl ProgramSupportApi for ProgramSupport {
    async fn compile(&self) -> Result<()> {
        match self {
            ProgramSupport::Dev(s) => s.compile().await,
            ProgramSupport::Risc0(s) => s.compile().await,
        }
    }
    async fn start(&self) -> Result<()> {
        match self {
            ProgramSupport::Dev(s) => s.start().await,
            ProgramSupport::Risc0(s) => s.start().await,
        }
    }
}

#[async_trait]
pub trait ProgramSupportApi {
    async fn compile(&self) -> Result<()>;
    async fn start(&self) -> Result<()>;
}

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
