use anyhow::{bail, Result};
use duct::cmd;
use std::{env, path::PathBuf};
use tokio::fs;
use tokio::signal;

async fn run_bash_script(cwd: &PathBuf, script: &PathBuf, args: &[&str]) -> Result<()> {
    println!("run_bash_script: {:?} {:?} {:?}", cwd, script, args);

    // Build the command using cmd! macro for cleaner syntax
    let mut cmd_args = vec!["bash".to_string(), script.to_string_lossy().to_string()];
    cmd_args.extend(args.iter().map(|s| s.to_string()));

    let expression = cmd("bash", &cmd_args[1..]).dir(cwd);

    let handle = expression.start()?;

    tokio::select! {
        result = async { handle.wait() } => {
            match result {
                Ok(output) => {
                    if output.status.success() {
                        Ok(())
                    } else {
                        bail!("{} failed with exit code: {:?}", script.display(), output.status.code());
                    }
                }
                Err(e) => Err(e.into()),
            }
        }
        _ = signal::ctrl_c() => {
            let _ = handle.kill();
            bail!("Script interrupted by user");
        }
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

pub enum SupportArgs {
    BonsaiCredentials { api_key: String, api_url: String },
    DevMode,
}

pub async fn program_start(bonsai_api: SupportArgs) -> Result<()> {
    let cwd = env::current_dir()?;
    let script = cwd.join(".enclave/support/ctl/start");
    ensure_script_exists(&script).await?;

    let args: Vec<&str> = match &bonsai_api {
        SupportArgs::BonsaiCredentials { api_key, api_url } => {
            vec!["--api-key", api_key.as_str(), "--api-url", api_url.as_str()]
        }
        SupportArgs::DevMode => vec![],
    };
    run_bash_script(&cwd, &script, &args).await?;
    Ok(())
}
