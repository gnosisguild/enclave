// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_noir_prover::{NoirSetup, SetupStatus};

#[derive(Subcommand, Debug)]
pub enum NoirCommands {
    Status,
    Setup {
        #[arg(long, short)]
        force: bool,
    },
}

pub async fn execute(command: NoirCommands, _config: &AppConfig) -> Result<()> {
    execute_without_config(command).await
}

pub async fn execute_without_config(command: NoirCommands) -> Result<()> {
    let setup = NoirSetup::with_default_dir()
        .await
        .map_err(|e| anyhow!("Failed to initialize noir setup: {}", e))?;

    match command {
        NoirCommands::Status => {
            execute_status(&setup).await?;
        }
        NoirCommands::Setup { force } => {
            execute_setup(&setup, force).await?;
        }
    }

    Ok(())
}

async fn execute_status(setup: &NoirSetup) -> Result<()> {
    let status = setup.check_status().await;
    let version_info = setup.load_version_info().await;

    println!("=== Noir Prover Status ===\n");

    println!("Barretenberg (bb):");
    println!("  Path: {}", setup.bb_binary.display());
    if let Some(ref v) = version_info.bb_version {
        println!("  Version: {}", v);
    }
    if setup.bb_binary.exists() {
        println!("  ✓ Installed");
    } else {
        println!("  ✗ Not installed");
    }

    println!();

    println!("Circuits:");
    println!("  Path: {}", setup.circuits_dir.display());
    if let Some(ref v) = version_info.circuits_version {
        println!("  Version: {}", v);
    }
    if setup.circuits_dir.exists() {
        println!("  ✓ Installed");
    } else {
        println!("  ✗ Not installed");
    }

    println!();

    match status {
        SetupStatus::Ready => {
            println!("Status: ✓ Ready");
        }
        SetupStatus::BbNeedsUpdate {
            installed,
            required,
        } => {
            println!("Status: ⚠ Barretenberg needs update");
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
            println!("Status: ⚠ Circuits need update");
            println!(
                "  Installed: {}",
                installed.as_deref().unwrap_or("not installed")
            );
            println!("  Required: {}", required);
            println!("\nRun `enclave noir setup` to update");
        }
        SetupStatus::FullSetupNeeded => {
            println!("Status: ✗ Setup required");
            println!("\nRun `enclave noir setup` to install");
        }
    }

    Ok(())
}

async fn execute_setup(setup: &NoirSetup, force: bool) -> Result<()> {
    if force {
        println!("Force reinstalling Noir prover components...\n");
    } else {
        let status = setup.check_status().await;
        if matches!(status, SetupStatus::Ready) {
            println!("✓ Noir prover is already set up and up to date.");
            println!("  Use --force to reinstall.");
            return Ok(());
        }
    }

    println!("Setting up Noir prover...\n");

    setup
        .ensure_installed()
        .await
        .map_err(|e| anyhow!("Setup failed: {}", e))?;

    println!("\n✓ Noir prover setup complete!");
    println!("  bb binary: {}", setup.bb_binary.display());
    println!("  circuits: {}", setup.circuits_dir.display());

    Ok(())
}
