mod copy;
mod file_utils;
mod git;
mod node_utils;
mod pkgman;

use anyhow::{bail, Result};
use async_recursion::async_recursion;
use copy::Filter;
use git::shallow_clone;
use node_utils::get_version_from_package_json;
use pkgman::PkgMan;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
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

    let version = node_utils::get_version_from_package_json(
        &PathBuf::from(temp_dir).join("packages/evm/package.json"),
    )
    .await?;

    copy::copy_with_filters(
        &PathBuf::from(temp_dir).join(template_folder),
        &cwd,
        &vec![
            Filter::new(".gitignore", "\\/deployments$", ""),
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

    let npm = PkgMan::new(pkgman::PkgManKind::PNPM)?.with_cwd(&cwd);

    npm.run(&["install"]).await?;

    git::init(&cwd).await?;

    Ok(())
}
