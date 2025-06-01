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
        .arg("-b")
        .arg("main")
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

pub async fn add_all(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .await
        .with_context(|| format!("Failed to execute git add in directory: {}", path.display()))?;
    Ok(())
}

pub async fn commit(path: impl AsRef<Path>, message: &str) -> Result<()> {
    let path = path.as_ref();
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(path)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to execute git commit in directory: {}",
                path.display()
            )
        })?;
    Ok(())
}

pub async fn add_submodule(
    repo_path: impl AsRef<Path>,
    submodule_url: &str,
    submodule_path: &str,
) -> Result<()> {
    let repo_path = repo_path.as_ref();
    Command::new("git")
        .arg("submodule")
        .arg("add")
        .arg(submodule_url)
        .arg(submodule_path)
        .current_dir(repo_path)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to add git submodule '{}' at '{}' in directory: {}",
                submodule_url,
                submodule_path,
                repo_path.display()
            )
        })?;
    Ok(())
}
