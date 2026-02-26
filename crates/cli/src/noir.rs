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
    let backend = ZkBackend::new(config.bb_binary(), config.circuits_dir(), config.work_dir());

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
    println!("Setting up ZK prover...\n");
    println!(
        "  target bb version:       {}",
        backend.config.required_bb_version
    );
    println!(
        "  target circuits version: {}\n",
        backend.config.required_circuits_version
    );

    if force {
        println!("Force reinstalling ZK prover components...\n");

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
            println!("ZK prover is already set up and up to date.");
            println!(
                "  bb version:         {}",
                version_info.bb_version.as_deref().unwrap_or("unknown")
            );
            println!(
                "  circuits version:   {}",
                version_info
                    .circuits_version
                    .as_deref()
                    .unwrap_or("unknown")
            );
            println!("  Use --force to reinstall.");
            return Ok(());
        }

        backend
            .ensure_installed()
            .await
            .map_err(|e| anyhow!("Setup failed: {}", e))?;
    }

    let version_info = backend.load_version_info().await;

    println!("\nZK prover setup complete!");
    println!();
    println!("  bb binary:          {}", backend.bb_binary.display());
    println!(
        "  bb version:         {}",
        version_info.bb_version.as_deref().unwrap_or("unknown")
    );
    println!("  circuits dir:       {}", backend.circuits_dir.display());
    println!(
        "  circuits version:   {}",
        version_info
            .circuits_version
            .as_deref()
            .unwrap_or("unknown")
    );
    if let Some(ref ts) = version_info.last_updated {
        println!("  last updated:       {}", ts);
    }

    Ok(())
}
