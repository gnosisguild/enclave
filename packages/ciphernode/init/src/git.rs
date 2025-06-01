use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;

pub async fn shallow_clone(git_repo: &str, branch: &str, target_folder: &str) -> Result<()> {
    Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            branch,
            git_repo,
            target_folder,
        ])
        .status()
        .await?;
    Ok(())
}

pub async fn init(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to execute git init in directory: {}",
                path.display()
            )
        })?;

    Ok(())
}
