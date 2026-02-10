// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_zk_prover::{SetupStatus, ZkBackend};

#[derive(Subcommand, Debug)]
pub enum NoirCommands {
    Status,
    Setup {
        #[arg(long, short)]
        force: bool,
    },
}

pub async fn execute(command: NoirCommands, config: &AppConfig) -> Result<()> {
    let zk_config = e3_zk_prover::ZkConfig::fetch_or_default().await;
    let backend = ZkBackend::new(
        config.bb_binary(),
        config.circuits_dir(),
        config.work_dir(),
        zk_config,
    );

    match command {
        NoirCommands::Status => {
            execute_status(&backend).await?;
        }
        NoirCommands::Setup { force } => {
            execute_setup(&backend, force).await?;
        }
    }

    Ok(())
}

pub async fn execute_without_config(command: NoirCommands) -> Result<()> {
    let backend = ZkBackend::with_default_dir("default")
        .await
        .map_err(|e| anyhow!("Failed to initialize ZK backend: {}", e))?;

    match command {
        NoirCommands::Status => {
            execute_status(&backend).await?;
        }
        NoirCommands::Setup { force } => {
            execute_setup(&backend, force).await?;
        }
    }

    Ok(())
}

async fn execute_status(backend: &ZkBackend) -> Result<()> {
    let status = backend.check_status().await;
    let version_info = backend.load_version_info().await;

    println!("=== ZK Prover Status ===\n");

    println!("Barretenberg (bb):");
    println!("  Path: {}", backend.bb_binary.display());
    if let Some(ref v) = version_info.bb_version {
        println!("  Version: {}", v);
    }
    if backend.bb_binary.exists() {
        println!("  Installed");
    } else {
        println!("  Not installed");
    }

    println!();

    println!("Circuits:");
    println!("  Path: {}", backend.circuits_dir.display());
    if let Some(ref v) = version_info.circuits_version {
        println!("  Version: {}", v);
    }
    if backend.circuits_dir.exists() {
        println!("  Installed");
    } else {
        println!("  Not installed");
    }

    println!();

    match status {
        SetupStatus::Ready => {
            println!("Status: Ready");
        }
        SetupStatus::BbNeedsUpdate {
            installed,
            required,
        } => {
            println!("Status: Barretenberg needs update");
            println!(
                "  Installed: {}",
                installed.as_deref().unwrap_or("not installed")
            );
            println!("  Required: {}", required);
            println!("\nRun `enclave noir setup` to update");
        }
        SetupStatus::CircuitsNeedUpdate {
            installed,
            required,
        } => {
            println!("Status: Circuits need update");
            println!(
                "  Installed: {}",
                installed.as_deref().unwrap_or("not installed")
            );
            println!("  Required: {}", required);
            println!("\nRun `enclave noir setup` to update");
        }
        SetupStatus::FullSetupNeeded => {
            println!("Status: Setup required");
            println!("\nRun `enclave noir setup` to install");
        }
    }

    Ok(())
}

async fn execute_setup(backend: &ZkBackend, force: bool) -> Result<()> {
    if force {
        println!("Force reinstalling ZK prover components...\n");
        println!("Setting up ZK prover...\n");

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
            println!("ZK prover is already set up and up to date.");
            println!("  Use --force to reinstall.");
            return Ok(());
        }

        println!("Setting up ZK prover...\n");

        backend
            .ensure_installed()
            .await
            .map_err(|e| anyhow!("Setup failed: {}", e))?;
    }

    println!("\nZK prover setup complete!");
    println!("  bb binary: {}", backend.bb_binary.display());
    println!("  circuits: {}", backend.circuits_dir.display());

    Ok(())
}
