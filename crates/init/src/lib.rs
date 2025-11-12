// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod copy;
mod file_utils;
mod git;
mod logging;
mod package_json;
mod pkgman;

use anyhow::Result;
use copy::Filter;
use file_utils::{chmod_recursive, delete_path, move_file, remove_all_files_in_dir};
use git::parse_git_url;
use package_json::DependencyType;
use pkgman::PkgMan;
use std::path::PathBuf;
use std::process::exit;
use std::{env, vec};
use tokio::fs;

use crate::logging::TaskSpinner;

const DEFAULT_TEMPLATE_URL: &str =
    "https://github.com/gnosisguild/enclave.git#v{{VERSION}}:templates/default";
const TEMP_DIR: &str = "/tmp/__enclave-tmp-folder.1";
const DEFAULT_TEMPLATE_PATH: &str = ".";
const DEFAULT_BRANCH: &str = "main";

async fn install_enclave(cwd: &PathBuf, template: Option<String>, verbose: bool) -> Result<()> {
    let mut spinner = TaskSpinner::new("".to_string(), verbose);

    spinner.update("Downloading template...".to_string()).await;

    let template_url = if let Some(template) = template {
        template
    } else {
        let version = env!("CARGO_PKG_VERSION").to_string();

        DEFAULT_TEMPLATE_URL.replace("{{VERSION}}", &version)
    };

    let repo = parse_git_url(template_url)?;
    let base_url = repo.base_url;
    let branch = repo.branch.unwrap_or(DEFAULT_BRANCH.to_string());
    let template_path = repo.path.unwrap_or(DEFAULT_TEMPLATE_PATH.to_string());

    spinner
        .run("Start git clone", || async {
            git::shallow_clone(&base_url, &branch, TEMP_DIR, verbose).await
        })
        .await?;

    let commit_hash = spinner
        .run("Getting commit hash...", || async {
            git::get_commit_hash(TEMP_DIR).await
        })
        .await?;

    spinner.complete_task(&format!(
        "Template downloaded with commit hash '{}'\n",
        commit_hash
    ));

    spinner.update("Configuring template...".to_string()).await;

    let evm_version = spinner
        .run("Getting workspace version of enclave...", || async {
            package_json::get_version_from_package_json(
                &PathBuf::from(TEMP_DIR).join("packages/enclave-contracts/package.json"),
            )
            .await
        })
        .await?;

    let react_version = spinner
        .run("Getting workspace version of enclave-react...", || async {
            package_json::get_version_from_package_json(
                &PathBuf::from(TEMP_DIR).join("packages/enclave-react/package.json"),
            )
            .await
        })
        .await?;

    let sdk_version = spinner
        .run("Getting workspace version of enclave-sdk...", || async {
            package_json::get_version_from_package_json(
                &PathBuf::from(TEMP_DIR).join("packages/enclave-sdk/package.json"),
            )
            .await
        })
        .await?;

    let src = PathBuf::from(TEMP_DIR).join(template_path);

    spinner
        .run("Copy with filters...", || async {
            copy::copy_with_filters(
                &src,
                &cwd,
                &vec![
                    Filter::new(
                        "**/package.json",
                        r#""@enclave-e3/contracts":\s*"[^"]*""#,
                        &format!(r#""@enclave-e3/contracts": "{}""#, evm_version),
                    ),
                    Filter::new(
                        "**/package.json",
                        r#""@enclave-e3/react":\s*"[^"]*""#,
                        &format!(r#""@enclave-e3/react": "{}""#, react_version),
                    ),
                    Filter::new(
                        "**/package.json",
                        r#""@enclave-e3/sdk":\s*"[^"]*""#,
                        &format!(r#""@enclave-e3/sdk": "{}""#, sdk_version),
                    ),
                    Filter::new(
                        "**/program/Cargo.toml",
                        r#"e3-compute-provider"#,
                        &format!("YO LATER DUDE {}", commit_hash),
                    ),
                ],
            )
            .await
        })
        .await?;

    spinner.complete_task("Template configured\n");

    spinner
        .update("Setting up support folders...".to_string())
        .await;

    spinner
        .run("Resetting support folder...", || async {
            delete_path(&cwd.join(".enclave")).await
        })
        .await?;

    spinner
        .run("Setting up support folders ctl and dev", || async {
            copy::copy_with_filters(
                &PathBuf::from(TEMP_DIR).join("crates/support-scripts/ctl"),
                &cwd.join(".enclave/support/ctl"),
                &vec![],
            )
            .await?;

            copy::copy_with_filters(
                &PathBuf::from(TEMP_DIR).join("crates/support-scripts/dev"),
                &cwd.join(".enclave/support/dev"),
                &vec![],
            )
            .await
        })
        .await?;

    spinner
        .run("Removing template ignore files...", || async {
            delete_path(&cwd.join(".gitignore")).await
        })
        .await?;

    spinner
        .run("Using bak files for ignores...", || async {
            move_file(&cwd.join(".gitignore.bak"), &cwd.join(".gitignore")).await
        })
        .await?;

    spinner
        .run("Move bak files for workspace...", || async {
            move_file(
                &cwd.join("pnpm-workspace.yaml.bak"),
                &cwd.join("pnpm-workspace.yaml"),
            )
            .await
        })
        .await?;

    spinner
        .run("Remove lib folder...", || async {
            delete_path(&cwd.join("lib")).await
        })
        .await?;

    spinner.complete_task("Support folders set up\n");

    // We need to make these chmod 777 because the dockerfile needs to be able to successfully
    // write to them. There are better ways to do this but right now this is the most efficient.
    // PRs/Ideas welcome.
    spinner.update("Restoring permissions...".to_string()).await;

    spinner
        .run("Setting contracts folder permissions to 777", || async {
            chmod_recursive(&cwd.join("contracts"), "777").await
        })
        .await?;
    spinner
        .run("Setting tests folder permissions to 777", || async {
            chmod_recursive(&cwd.join("tests"), "777").await
        })
        .await?;

    spinner.complete_task("Permissions restored\n");

    spinner.update("Setting up submodules...").await;

    spinner
        .run("Init git repo", || async { git::init(&cwd, verbose).await })
        .await?;

    spinner
        .run("Adding @risc0/ethereum submodule", || async {
            git::add_submodule(
                &cwd,
                "https://github.com/gnosisguild/risc0-ethereum",
                "lib/risc0-ethereum",
                verbose,
            )
            .await
        })
        .await?;

    spinner
        .run("Ensuring @risc0/ethereum is in package.json", || async {
            package_json::add_package_to_json(
                &cwd.join("package.json"),
                "@risc0/ethereum",
                "file:lib/risc0-ethereum",
                DependencyType::DevDependencies,
            )
            .await
        })
        .await?;

    spinner.complete_task("Submodules set up\n");

    spinner
        .update("Installing packages with pnpm...".to_string())
        .await;

    spinner
        .run("", || async {
            let npm = PkgMan::new(pkgman::PkgManKind::PNPM)?.with_cwd(&cwd);
            let mut args = vec!["install"];
            if !verbose {
                args.push("--silent");
            }
            npm.run(&args).await
        })
        .await?;

    spinner.complete_task("Packages installed\n");

    spinner.update("Setting up local git repository...").await;

    spinner
        .run("Adding all files to git", || async {
            git::add_all(&cwd, verbose).await
        })
        .await?;
    spinner
        .run("Committing changes", || async {
            git::commit(&cwd, "Initial Commit", verbose).await
        })
        .await?;

    git::add_all(&cwd, verbose).await?;

    git::commit(&cwd, "Initial Commit", verbose).await?;

    spinner.complete_task("Git repository set up\n");

    spinner.done("🎉 You can now start building on Enclave");

    Ok(())
}

// Updated execute function to include workspace dependency substitution
pub async fn execute(
    location: Option<PathBuf>,
    template: Option<String>,
    skip_cleanup: bool,
    verbose: bool,
) -> Result<()> {
    println!(
        r"
    ░██████████                       ░██                                  
    ░██                               ░██                                  
    ░██         ░████████   ░███████  ░██  ░██████   ░██    ░██  ░███████  
    ░█████████  ░██    ░██ ░██    ░██ ░██       ░██  ░██    ░██ ░██    ░██ 
    ░██         ░██    ░██ ░██        ░██  ░███████   ░██  ░██  ░█████████ 
    ░██         ░██    ░██ ░██    ░██ ░██ ░██   ░██    ░██░██   ░██        
    ░██████████ ░██    ░██  ░███████  ░██  ░█████░██    ░███     ░███████                                                                                                                                                     
    "
    );

    let mut task_spinner =
        TaskSpinner::new("Setting up a new Enclave project".to_string(), verbose);

    task_spinner.update("Preparing paths...".to_string()).await;

    let mut install_in_current_dir = false;
    let env_current_dir = env::current_dir()?;
    let cwd = match location {
        Some(loc) => {
            if loc.is_absolute() {
                loc
            } else {
                env_current_dir.join(loc)
            }
        }
        None => {
            install_in_current_dir = true;
            env_current_dir
        }
    };

    task_spinner
        .run("Ensuring tmp dir does not exists...", || async {
            if fs::try_exists(TEMP_DIR).await? {
                fs::remove_dir_all(TEMP_DIR).await?;
            }
            Ok(())
        })
        .await?;

    task_spinner
        .run("Ensuring current directory is empty...", || async {
            file_utils::ensure_empty_folder(&cwd).await?;

            Ok(())
        })
        .await?;

    task_spinner.complete_task("Paths prepared");
    task_spinner.done("");

    match install_enclave(&cwd, template, verbose).await {
        Ok(_) => Ok(()),
        Err(e) => {
            if !skip_cleanup {
                println!("❌ Cleaning up due to error...");
                if install_in_current_dir {
                    remove_all_files_in_dir(&cwd).await?;
                } else {
                    fs::remove_dir_all(&cwd).await?;
                }
            }
            eprintln!("❌ Sorry about this but there was an error running the installer. ");
            eprintln!("❌ Error: {}\n", e);
            eprintln!("Enclave is currently under active development please share this with our team:\n\n  https://github.com/gnosisguild/enclave/issues/new\n");
            exit(1);
        }
    }
}
