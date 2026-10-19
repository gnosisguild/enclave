// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{anyhow, bail, Result};
use duct::cmd;
use std::path::Path;
use tokio::fs;
use tokio::signal;

/// A support script that the CLI carries. It runs in the project directory and builds or starts
/// the project's runner, the binary that serves the project's program through `e3-program-runner`.
pub struct Script {
    pub name: &'static str,
    pub source: &'static str,
}

pub const DEV_COMPILE: Script = Script {
    name: "dev/compile",
    source: include_str!("../dev/compile"),
};
pub const DEV_START: Script = Script {
    name: "dev/start",
    source: include_str!("../dev/start"),
};
pub const OPENVM_COMPILE: Script = Script {
    name: "openvm/compile",
    source: include_str!("../openvm/compile"),
};
pub const OPENVM_START: Script = Script {
    name: "openvm/start",
    source: include_str!("../openvm/start"),
};

pub async fn run_script(
    cwd: &Path,
    script: &Script,
    environment: &[(String, String)],
) -> Result<()> {
    // `bash -c <source> <name>` names the script `$0`. `duct` puts every argument, the source
    // included, in its checked-process error, so inspect the exit status here and name only the
    // script.
    let mut expression = cmd("bash", ["-c", script.source, script.name])
        .dir(cwd)
        .unchecked();
    for (name, value) in environment {
        expression = expression.env(name, value);
    }

    let handle = expression
        .start()
        .map_err(|_| anyhow!("failed to start {}", script.name))?;

    tokio::select! {
        result = async { handle.wait() } => {
            match result {
                Ok(output) => {
                    if output.status.success() {
                        Ok(())
                    } else {
                        bail!("{} failed with exit code: {:?}", script.name, output.status.code());
                    }
                }
                Err(_) => Err(anyhow!("failed while waiting for {}", script.name)),
            }
        }
        _ = signal::ctrl_c() => {
            let _ = handle.kill();
            bail!("Script interrupted by user");
        }
    }
}

/// Checks that `cwd` is a project that the support scripts can build.
pub async fn ensure_project(cwd: &Path) -> Result<()> {
    if fs::try_exists(cwd.join(".interfold/support")).await? {
        bail!(
            "This project keeps its program server in .interfold/support, which the interfold CLI \
             no longer uses. Delete .interfold/support and its Cargo workspace members, then add \
             the runner crate from the project template (templates/default/runner)."
        );
    }
    if !fs::try_exists(cwd.join("Cargo.toml")).await? {
        bail!("Invalid or corrupted project. This command can only be run from within a valid Interfold project.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    async fn temp_dir() -> Result<PathBuf> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "interfold-support-script-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).await?;
        Ok(directory)
    }

    #[tokio::test]
    async fn passes_configuration_through_the_child_environment() -> Result<()> {
        let directory = temp_dir().await?;
        let script = Script {
            name: "check-env",
            source: "test \"$PRIVATE_KEY\" = \"credential-from-environment\"\n",
        };

        run_script(
            &directory,
            &script,
            &[(
                "PRIVATE_KEY".to_owned(),
                "credential-from-environment".to_owned(),
            )],
        )
        .await?;
        let failing = Script {
            name: "fail",
            source: "exit 17\n",
        };
        let error = run_script(&directory, &failing, &[])
            .await
            .expect_err("the script must fail");
        assert_eq!(error.to_string(), "fail failed with exit code: Some(17)");
        fs::remove_dir_all(directory).await?;
        Ok(())
    }

    #[tokio::test]
    async fn refuses_a_project_with_the_removed_support_folder() -> Result<()> {
        let directory = temp_dir().await?;
        fs::write(directory.join("Cargo.toml"), "").await?;
        ensure_project(&directory).await?;

        fs::create_dir_all(directory.join(".interfold/support/dev")).await?;
        let error = ensure_project(&directory)
            .await
            .expect_err("the old layout must be refused");
        assert!(error.to_string().contains("templates/default/runner"));
        fs::remove_dir_all(directory).await?;
        Ok(())
    }
}
