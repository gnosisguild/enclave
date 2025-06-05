mod copy;
mod file_utils;
mod git;
mod git_url;
mod package_json;
mod pkgman;

use anyhow::Result;
use copy::Filter;
use file_utils::{chmod_recursive, delete_path, move_file};
use git_url::GitUrl;
use package_json::DependencyType;
use pkgman::PkgMan;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::fs;

const GIT_URL: &str = "https://github.com/gnosisguild/enclave.git#ry/support-alterations-1";
// const GIT_URL: &str = "https://github.com/gnosisguild/enclave.git#hacknet";
const TEMPLATE_FOLDER: &str = "templates/default";
const TEMP_DIR: &str = "/tmp/__enclave-tmp-folder.1";

// Updated execute function to include workspace dependency substitution
pub async fn execute(location: Option<PathBuf>) -> Result<()> {
    let repo = GitUrl::from_str(GIT_URL)?;

    let cwd = match location {
        Some(loc) => loc,
        None => env::current_dir()?,
    };

    println!("Ensuring tmp folder does not exist...");
    if fs::try_exists(TEMP_DIR).await? {
        fs::remove_dir_all(TEMP_DIR).await?;
    }
    println!("Ensuring cwd is empty...");
    file_utils::ensure_empty_folder(&cwd).await?;

    println!("Start git clone...");
    git::shallow_clone(&repo.repo_url, &repo.branch, TEMP_DIR).await?;

    println!("Getting workspace version for enclave...");
    let evm_version = package_json::get_version_from_package_json(
        &PathBuf::from(TEMP_DIR).join("packages/evm/package.json"),
    )
    .await?;

    println!("Getting workspace version for enclave_react...");
    let react_version = package_json::get_version_from_package_json(
        &PathBuf::from(TEMP_DIR).join("packages/enclave-react/package.json"),
    )
    .await?;

    println!("Copy with filters...");
    copy::copy_with_filters(
        &PathBuf::from(TEMP_DIR).join(TEMPLATE_FOLDER),
        &cwd,
        &vec![
            Filter::new(
                "package.json",
                "\"@gnosis-guild/enclave\":\\s*\"[^\"]*\"",
                &format!("\"@gnosis-guild/enclave\": \"{}\"", evm_version),
            ),
            Filter::new(
                "package.json",
                "\"@gnosis-guild/enclave-react\":\\s*\"[^\"]*\"",
                &format!("\"@gnosis-guild/enclave-react\": \"{}\"", react_version),
            ),
        ],
    )
    .await?;

    println!("Resetting support folder...");
    delete_path(&cwd.join(".enclave")).await?;

    println!("Setting up support folders...");
    copy::copy_with_filters(
        &PathBuf::from(TEMP_DIR).join("crates/support-scripts/ctl"),
        &cwd.join(".enclave/support/ctl"),
        &vec![],
    )
    .await?;

    println!("Removing template ignore files...");
    delete_path(&cwd.join(".gitignore")).await?;

    println!("Using bak files for ignores...");
    move_file(&cwd.join(".gitignore.bak"), &cwd.join(".gitignore")).await?;

    println!("Remove lib folder...");
    delete_path(&cwd.join("lib")).await?;

    // We need to make these chmod 777 because the dockerfile needs to be able to successfully
    // write to them. There are better ways to do this but right now this is the most efficient.
    // PRs/Ideas welcome.
    //
    println!("Fixing permissions...");
    chmod_recursive(&cwd.join("contracts"), "777").await?;
    chmod_recursive(&cwd.join("tests"), "777").await?;

    println!("Initializing repository...");
    git::init(&cwd).await?;

    println!("Setting up submodule...");
    git::add_submodule(
        &cwd,
        "https://github.com/gnosisguild/risc0-ethereum",
        "lib/risc0-ethereum",
    )
    .await?;

    println!("Ensuring package is in json...");
    package_json::add_package_to_json(
        &cwd.join("package.json"),
        "@risc0/ethereum",
        "file:lib/risc0-ethereum",
        DependencyType::DevDependencies,
    )
    .await?;

    println!("Running pnpm install...");
    let npm = PkgMan::new(pkgman::PkgManKind::PNPM)?.with_cwd(&cwd);
    npm.run(&["install"]).await?;

    println!("Adding everything to git...");
    git::add_all(&cwd).await?;

    println!("Committing...");
    git::commit(&cwd, "Initial Commit").await?;

    Ok(())
}
