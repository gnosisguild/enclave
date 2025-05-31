use anyhow::{bail, Result};
use async_recursion::async_recursion;
use git2::{FetchOptions, IndexAddOption, Repository, Signature};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use tokio::fs;
use tokio::process::Command as TokioCommand;

/// Substitutes "workspace:*" dependencies with actual versions from packages
pub async fn substitute_workspace_dependencies(
    target_package_json: &Path,
    packages_dir: &Path,
) -> Result<()> {
    println!("Substituting workspace dependencies...");

    // Read the target package.json
    let target_content = fs::read_to_string(target_package_json).await?;
    let mut target_json: Value = serde_json::from_str(&target_content)?;

    // Build a map of package names to versions from the packages directory
    let package_versions = collect_package_versions(packages_dir).await?;

    // Substitute workspace dependencies in all dependency sections
    let dependency_sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];

    let mut substitutions_made = false;

    for section in &dependency_sections {
        if let Some(deps) = target_json.get_mut(section) {
            if let Some(deps_obj) = deps.as_object_mut() {
                for (package_name, version) in deps_obj.iter_mut() {
                    if version.as_str() == Some("workspace:*") {
                        if let Some(actual_version) = package_versions.get(package_name) {
                            *version = Value::String(actual_version.clone());
                            println!(
                                "  {} -> {}: workspace:* -> {}",
                                section, package_name, actual_version
                            );
                            substitutions_made = true;
                        } else {
                            bail!("Package '{}' with workspace:* dependency not found in packages directory", package_name);
                        }
                    }
                }
            }
        }
    }

    if substitutions_made {
        // Write the updated package.json back
        let updated_content = serde_json::to_string_pretty(&target_json)?;
        fs::write(target_package_json, updated_content).await?;
        println!("Updated package.json with actual versions");
    } else {
        println!("No workspace:* dependencies found to substitute");
    }

    Ok(())
}

/// Recursively collects package names and versions from all package.json files in packages directory
async fn collect_package_versions(packages_dir: &Path) -> Result<HashMap<String, String>> {
    let mut package_versions = HashMap::new();

    if !packages_dir.exists() {
        bail!(
            "Packages directory '{}' does not exist",
            packages_dir.display()
        );
    }

    collect_versions_recursive(packages_dir, &mut package_versions).await?;

    if package_versions.is_empty() {
        println!("Warning: No packages found in {}", packages_dir.display());
    } else {
        println!("Found {} packages:", package_versions.len());
        for (name, version) in &package_versions {
            println!("  {} -> {}", name, version);
        }
    }

    Ok(package_versions)
}

/// Recursively walks through directories to find package.json files
#[async_recursion]
async fn collect_versions_recursive(
    dir: &Path,
    package_versions: &mut HashMap<String, String>,
) -> Result<()> {
    let mut entries = fs::read_dir(dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_dir() {
            // Skip node_modules and hidden directories
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if !dir_name.starts_with('.') && dir_name != "node_modules" {
                    collect_versions_recursive(&path, package_versions).await?;
                }
            }
        } else if path.file_name() == Some(std::ffi::OsStr::new("package.json")) {
            // Found a package.json, extract name and version
            if let Ok((name, version)) = extract_package_info(&path).await {
                package_versions.insert(name, version);
            }
        }
    }

    Ok(())
}

/// Extracts package name and version from a package.json file
async fn extract_package_info(package_json_path: &Path) -> Result<(String, String)> {
    let content = fs::read_to_string(package_json_path).await?;
    let json: Value = serde_json::from_str(&content)?;

    let name = json
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!("Package name not found in {}", package_json_path.display())
        })?
        .to_string();

    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Package version not found in {}",
                package_json_path.display()
            )
        })?
        .to_string();

    Ok((name, version))
}

// Updated execute function to include workspace dependency substitution
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
            // Also copy the packages directory so we can read versions
            ("packages", ".enclave/packages"),
        ],
        branch,
        temp_dir,
    )
    .await?;

    // Substitute workspace dependencies
    let target_package_json = cwd.join("package.json");
    let packages_dir = cwd.join(".enclave/packages");

    substitute_workspace_dependencies(&target_package_json, &packages_dir).await?;

    // Clean up the temporary packages directory
    if packages_dir.exists() {
        tokio::fs::remove_dir_all(&packages_dir).await?;
    }

    Pnpm::run(&["install"]).await?;
    init_git_repo_if_needed(&cwd).await?;

    Ok(())
}

// Add this to your Cargo.toml dependencies:
// serde_json = "1.0"

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
