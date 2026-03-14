// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_console::Out;
use e3_zk_prover::{SetupStatus, ZkBackend};

#[derive(Subcommand, Debug)]
pub enum NoirCommands {
    Status,
    Setup {
        #[arg(long, short)]
        force: bool,
    },
}

pub async fn execute(out: Out, command: NoirCommands, config: &AppConfig) -> Result<()> {
    let backend = ZkBackend::new(config.bb_binary(), config.circuits_dir(), config.work_dir());

    match command {
        NoirCommands::Status => {
            execute_status(out, &backend).await?;
        }
        NoirCommands::Setup { force } => {
            execute_setup(out, &backend, force).await?;
        }
    }

    Ok(())
}

pub async fn execute_without_config(out: Out, command: NoirCommands) -> Result<()> {
    let backend = ZkBackend::with_default_dir("default")
        .map_err(|e| anyhow!("Failed to initialize ZK backend: {}", e))?;

    match command {
        NoirCommands::Status => {
            execute_status(out, &backend).await?;
        }
        NoirCommands::Setup { force } => {
            execute_setup(out, &backend, force).await?;
        }
    }

    Ok(())
}

async fn execute_status(out: Out, backend: &ZkBackend) -> Result<()> {
    let status = backend.check_status().await;
    let version_info = backend.load_version_info().await;

    e3_console::log!(out, "=== ZK Prover Status ===\n");

    e3_console::log!(out, "Barretenberg (bb):");
    e3_console::log!(out, "  Path: {}", backend.bb_binary.display());
    if let Some(ref v) = version_info.bb_version {
        e3_console::log!(out, "  Version: {}", v);
    }
    if backend.bb_binary.exists() {
        e3_console::log!(out, "  Installed");
    } else {
        e3_console::log!(out, "  Not installed");
    }

    e3_console::log!(out, "");

    e3_console::log!(out, "Circuits:");
    e3_console::log!(out, "  Path: {}", backend.circuits_dir.display());
    if let Some(ref v) = version_info.circuits_version {
        e3_console::log!(out, "  Version: {}", v);
    }
    if backend.circuits_dir.exists() {
        e3_console::log!(out, "  Installed");
    } else {
        e3_console::log!(out, "  Not installed");
    }

    e3_console::log!(out, "");

    match status {
        SetupStatus::Ready => {
            e3_console::log!(out, "Status: Ready");
        }
        SetupStatus::BbNeedsUpdate {
            installed,
            required,
        } => {
            e3_console::log!(out, "Status: Barretenberg needs update");
            e3_console::log!(
                out,
                "  Installed: {}",
                installed.as_deref().unwrap_or("not installed")
            );
            e3_console::log!(out, "  Required: {}", required);
            e3_console::log!(out, "\nRun `enclave noir setup` to update");
        }
        SetupStatus::CircuitsNeedUpdate {
            installed,
            required,
        } => {
            e3_console::log!(out, "Status: Circuits need update");
            e3_console::log!(
                out,
                "  Installed: {}",
                installed.as_deref().unwrap_or("not installed")
            );
            e3_console::log!(out, "  Required: {}", required);
            e3_console::log!(out, "\nRun `enclave noir setup` to update");
        }
        SetupStatus::FullSetupNeeded => {
            e3_console::log!(out, "Status: Setup required");
            e3_console::log!(out, "\nRun `enclave noir setup` to install");
        }
    }

    Ok(())
}

async fn execute_setup(out: Out, backend: &ZkBackend, force: bool) -> Result<()> {
    e3_console::log!(out, "Setting up ZK prover...\n");
    e3_console::log!(
        out,
        "  target bb version:       {}",
        backend.config.required_bb_version
    );
    e3_console::log!(
        out,
        "  target circuits version: {}\n",
        backend.config.required_circuits_version
    );

    if force {
        e3_console::log!(out, "Force reinstalling ZK prover components...\n");

        // Force reinstall by directly downloading components
        backend
            .download_bb()
            .await
            .map_err(|e| anyhow!("Failed to download bb: {}", e))?;
        backend
            .download_circuits()
            .await
            .map_err(|e| anyhow!("Failed to download circuits: {}", e))?;
    } else {
        let status = backend.check_status().await;
        if matches!(status, SetupStatus::Ready) {
            let version_info = backend.load_version_info().await;
            e3_console::log!(out, "ZK prover is already set up and up to date.");
            e3_console::log!(
                out,
                "  bb version:         {}",
                version_info.bb_version.as_deref().unwrap_or("unknown")
            );
            e3_console::log!(
                out,
                "  circuits version:   {}",
                version_info
                    .circuits_version
                    .as_deref()
                    .unwrap_or("unknown")
            );
            e3_console::log!(out, "  Use --force to reinstall.");
            return Ok(());
        }

        backend
            .ensure_installed()
            .await
            .map_err(|e| anyhow!("Setup failed: {}", e))?;
    }

    let version_info = backend.load_version_info().await;

    e3_console::log!(out, "\nZK prover setup complete!");
    e3_console::log!(out, "");
    e3_console::log!(out, "  bb binary:          {}", backend.bb_binary.display());
    e3_console::log!(
        out,
        "  bb version:         {}",
        version_info.bb_version.as_deref().unwrap_or("unknown")
    );
    e3_console::log!(
        out,
        "  circuits dir:       {}",
        backend.circuits_dir.display()
    );
    e3_console::log!(
        out,
        "  circuits version:   {}",
        version_info
            .circuits_version
            .as_deref()
            .unwrap_or("unknown")
    );
    if let Some(ref ts) = version_info.last_updated {
        e3_console::log!(out, "  last updated:       {}", ts);
    }

    Ok(())
}
