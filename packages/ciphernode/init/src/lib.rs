mod copy;
mod file_utils;
mod git;
mod package_json;
mod pkgman;

use anyhow::Result;
use copy::Filter;
use package_json::DependencyType;
use pkgman::PkgMan;
use std::env;
use std::path::PathBuf;
use tokio::fs;

// Updated execute function to include workspace dependency substitution
pub async fn execute(location: Option<PathBuf>) -> Result<()> {
    let github_repo = "https://github.com/gnosisguild/enclave.git";
    let template_folder = "examples/basic";
    let branch = "ry/389-enclave-init-3";
    let temp_dir = "/tmp/enclave-basic-example";

    let cwd = match location {
        Some(loc) => loc,
        None => env::current_dir()?,
    };
    fs::remove_dir_all(temp_dir).await?;
    file_utils::ensure_empty_folder(&cwd).await?;
    git::shallow_clone(github_repo, branch, temp_dir).await?;

    let version = package_json::get_version_from_package_json(
        &PathBuf::from(temp_dir).join("packages/evm/package.json"),
    )
    .await?;

    copy::copy_with_filters(
        &PathBuf::from(temp_dir).join(template_folder),
        &cwd,
        &vec![
            Filter::new(".gitignore", "/deployments$", ""),
            Filter::new("package.json", "workspace:\\*", &version),
        ],
    )
    .await?;

    copy::copy_with_filters(
        &PathBuf::from(temp_dir).join("packages/ciphernode/init/templates/support"),
        &cwd.join(".enclave/support"),
        &vec![],
    )
    .await?;

    git::init(&cwd).await?;

    git::add_submodule(
        &cwd,
        "https://github.com/risc0/risc0-ethereum",
        "lib/risc0-ethereum",
    )
    .await?;

    package_json::add_package_to_json(
        &cwd.join("package.json"),
        "@risc0/ethereum",
        "file:lib/risc0-ethereum",
        DependencyType::DevDependencies,
    )
    .await?;

    let npm = PkgMan::new(pkgman::PkgManKind::PNPM)?.with_cwd(&cwd);
    npm.run(&["install"]).await?;

    git::add_all(&cwd).await?;
    git::commit(&cwd, "Initial Commit").await?;

    Ok(())
}
