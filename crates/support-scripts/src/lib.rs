mod program;
mod program_dev;
mod program_risc0;
mod traits;
mod utils;

use anyhow::Result;
use e3_config::ProgramConfig;
use program::ProgramSupport;
use std::env;
use tokio::fs;
use traits::ProgramSupportApi;
use utils::{ensure_script_exists, run_bash_script};

pub async fn program_compile(program_config: ProgramConfig, is_dev: Option<bool>) -> Result<()> {
    ProgramSupport::new(program_config, is_dev).compile().await
}

pub async fn program_start(program_config: ProgramConfig, is_dev: Option<bool>) -> Result<()> {
    ProgramSupport::new(program_config, is_dev).start().await
}

/// Open up a shell in the docker container
pub async fn program_shell() -> Result<()> {
    let cwd = env::current_dir()?;
    let script = cwd.join(".enclave/support/ctl/shell");
    ensure_script_exists(&script).await?;
    run_bash_script(&cwd, &script, &[]).await?;
    Ok(())
}

/// Purge all build caches from support
pub async fn program_cache_purge() -> Result<()> {
    let cwd = env::current_dir()?;
    let caches = cwd.join(".enclave/caches");
    fs::remove_dir_all(caches).await?;
    Ok(())
}
