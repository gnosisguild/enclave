use anyhow::{bail, Result};
use async_recursion::async_recursion;
use git2::FetchOptions;
use std::env;
use std::path::Path;
use tokio::process::Command as TokioCommand;

pub async fn execute() -> Result<()> {
    let github_repo = "https://github.com/gnosisguild/enclave.git";
    let template_folder = "examples/basic";
    let branch = "ry/389-enclave-init-crisp";
    let temp_dir = "/tmp/enclave-basic-example";
    clone_repo(github_repo, template_folder, branch, temp_dir).await?;
    Pnpm::run(&["install"]).await?;
    Ok(())
}

pub struct Pnpm;

impl Pnpm {
    pub async fn is_available() -> bool {
        TokioCommand::new("pnpm")
            .arg("--version")
            .output()
            .await
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    pub async fn run(args: &[&str]) -> Result<()> {
        let cwd = env::current_dir()?;
        Self::run_in_dir(&cwd, args).await
    }

    pub async fn run_in_dir<P: AsRef<Path>>(dir: P, args: &[&str]) -> Result<()> {
        if !Self::is_available().await {
            bail!("pnpm is not installed or not available in PATH");
        }

        let status = TokioCommand::new("pnpm")
            .args(args)
            .current_dir(dir)
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            bail!("pnpm command failed with exit code: {:?}", status.code());
        }
    }
}

async fn clone_repo(
    github_repo: &str,
    template_folder: &str,
    branch: &str,
    temp_dir: &str,
) -> Result<()> {
    if Path::new(temp_dir).exists() {
        tokio::fs::remove_dir_all(temp_dir).await?;
    }

    println!("Cloning repository...");
    let mut fetch_options = FetchOptions::new();
    fetch_options.download_tags(git2::AutotagOption::None);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);
    builder.branch(branch);
    builder.clone(github_repo, Path::new(temp_dir))?;

    let source_path = Path::new(temp_dir).join(template_folder);

    if !source_path.exists() {
        anyhow::bail!(
            "Template folder '{}' not found in repository",
            template_folder
        );
    }

    // Get current working directory
    let cwd = std::env::current_dir()?;

    // Copy contents using async filesystem operations
    println!("Copying template contents to current directory...");
    copy_dir_contents_async(&source_path, &cwd).await?;

    // Clean up temporary directory
    tokio::fs::remove_dir_all(temp_dir).await?;

    println!("Template copied successfully!");
    Ok(())
}

#[async_recursion]
async fn copy_dir_contents_async(src: &Path, dst: &Path) -> Result<()> {
    let mut entries = tokio::fs::read_dir(src).await?;

    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            tokio::fs::create_dir_all(&dst_path).await?;
            copy_dir_contents_async(&src_path, &dst_path).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}
