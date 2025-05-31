use anyhow::{bail, Result};
use async_recursion::async_recursion;
use git2::{FetchOptions, IndexAddOption, Repository, Signature};
use std::env;
use std::path::Path;
use tokio::process::Command as TokioCommand;

pub async fn execute() -> Result<()> {
    let github_repo = "https://github.com/gnosisguild/enclave.git";
    let template_folder = "examples/basic";
    let branch = "ry/389-enclave-init-3";
    let temp_dir = "/tmp/enclave-basic-example";
    let cwd = env::current_dir()?;
    check_empty_folder(&cwd)?;
    clone_repo(
        github_repo,
        &[
            (template_folder, "."), // Copy to current directory
            (
                "packages/ciphernode/init/templates/support",
                ".enclave/support",
            ),
        ],
        branch,
        temp_dir,
    )
    .await?;
    Pnpm::run(&["install"]).await?;
    init_git_repo_if_needed(&cwd).await?;
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

fn is_git_repository<P: AsRef<Path>>(dir: P) -> bool {
    Repository::open(dir.as_ref()).is_ok()
}

async fn init_git_repo_if_needed<P: AsRef<Path>>(dir: P) -> Result<()> {
    let dir = dir.as_ref();

    // Check if directory is already a git repository
    if is_git_repository(dir) {
        println!("Directory is already a git repository, skipping initialization.");
        return Ok(());
    }

    println!("Initializing git repository...");

    // Initialize new git repository
    let repo = Repository::init(dir)?;

    // Get the repository index
    let mut index = repo.index()?;

    // Add all files to the index
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
    index.write()?;

    // Create the tree from the index
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    // Create signature for the commit
    let signature = Signature::now("Enclave Init", "developers@enclave.gg")?;

    // Create the initial commit
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial commit",
        &tree,
        &[],
    )?;

    println!("Git repository initialized with initial commit.");
    Ok(())
}

pub fn check_empty_folder<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        bail!("Path '{}' does not exist", path.display());
    }
    if !path.is_dir() {
        bail!("Path '{}' is not a directory", path.display());
    }

    let mut entries = std::fs::read_dir(path)
        .map_err(|e| anyhow::anyhow!("Failed to read directory '{}': {}", path.display(), e))?;

    if entries.next().is_some() {
        bail!("Directory '{}' is not empty", path.display());
    }

    Ok(())
}

async fn clone_repo(
    github_repo: &str,
    copy_operations: &[(&str, &str)],
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

    // Get current working directory
    let cwd = std::env::current_dir()?;

    // Perform all copy operations
    for (source, destination) in copy_operations {
        let source_path = Path::new(temp_dir).join(source);

        if !source_path.exists() {
            anyhow::bail!("Source path '{}' not found in repository", source);
        }

        let destination_path = if *destination == "." {
            cwd.clone()
        } else {
            cwd.join(destination)
        };

        println!("Copying '{}' to '{}'...", source, destination);

        // Create destination directory if it doesn't exist
        if let Some(parent) = destination_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // If destination is a directory that should contain the contents
        if source_path.is_dir() {
            if *destination == "." {
                // Copy contents to current directory
                copy_dir_contents_async(&source_path, &destination_path).await?;
            } else {
                // Create the destination directory and copy contents
                tokio::fs::create_dir_all(&destination_path).await?;
                copy_dir_contents_async(&source_path, &destination_path).await?;
            }
        } else {
            // Copy single file
            tokio::fs::copy(&source_path, &destination_path).await?;
        }
    }

    // Clean up temporary directory
    tokio::fs::remove_dir_all(temp_dir).await?;

    println!("All templates copied successfully!");
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
