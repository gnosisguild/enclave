use std::{env, path::PathBuf};

use anyhow::{bail, Result};
use tokio::{fs, process::Command};

async fn run_bash_script(cwd: &PathBuf, script: &PathBuf, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("bash");
    cmd.current_dir(cwd).arg(script);

    for arg in args {
        cmd.arg(arg);
    }

    let status = cmd.status().await?;

    if status.success() {
        Ok(())
    } else {
        bail!(
            "{} failed with exit code: {:?}",
            script.display(),
            status.code()
        );
    }
}

async fn ensure_script_exists(script_path: &PathBuf) -> Result<()> {
    if !fs::try_exists(script_path).await? {
        bail!("Invalid or corrupted project. This command can only be run from within a valid Enclave project.");
    }
    Ok(())
}

pub async fn program_compile() -> Result<()> {
    let cwd = env::current_dir()?;
    let script = cwd.join(".enclave/support/ctl/compile");
    ensure_script_exists(&script).await?;
    run_bash_script(&cwd, &script, &[]).await?;
    Ok(())
}

async fn ctl_run() -> Result<()> {
    let cwd = env::current_dir()?;
    let script = cwd.join(".enclave/support/ctl/run");
    ensure_script_exists(&script).await?;
    run_bash_script(&cwd, &script, &[]).await?;
    Ok(())
}

pub async fn program_listen() -> Result<()> {
    ctl_run().await?;
    Ok(())
}
