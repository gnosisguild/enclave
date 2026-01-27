// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_noir_prover::{NoirProver, NoirSetup, SetupStatus};

#[derive(Subcommand, Debug)]
pub enum NoirCommands {
    /// Check Noir prover setup status (bb binary and circuits)
    Status,

    /// Download or update bb binary and circuit artifacts
    Setup {
        /// Force re-download even if already installed
        #[arg(long, short)]
        force: bool,
    },

    /// Generate a proof for a circuit (for testing)
    Prove {
        /// Circuit name (pk-bfv, dec-share-trbfv, verify-shares)
        #[arg(long, short)]
        circuit: String,

        /// Path to input TOML file
        #[arg(long, short)]
        inputs: String,

        /// E3 ID for work directory isolation
        #[arg(long)]
        e3_id: String,
    },

    /// Verify a proof (for testing)
    Verify {
        /// Circuit name
        #[arg(long, short)]
        circuit: String,

        /// Path to proof file
        #[arg(long, short)]
        proof: String,

        /// E3 ID for work directory
        #[arg(long)]
        e3_id: String,
    },
}

pub async fn execute(command: NoirCommands, _config: &AppConfig) -> Result<()> {
    execute_without_config(command).await
}

pub async fn execute_without_config(command: NoirCommands) -> Result<()> {
    let setup = NoirSetup::with_default_dir()
        .map_err(|e| anyhow!("Failed to initialize noir setup: {}", e))?;

    match command {
        NoirCommands::Status => {
            execute_status(&setup).await?;
        }
        NoirCommands::Setup { force } => {
            execute_setup(&setup, force).await?;
        }
        NoirCommands::Prove {
            circuit,
            inputs,
            e3_id,
        } => {
            execute_prove(&setup, &circuit, &inputs, &e3_id).await?;
        }
        NoirCommands::Verify {
            circuit,
            proof,
            e3_id,
        } => {
            execute_verify(&setup, &circuit, &proof, &e3_id).await?;
        }
    }

    Ok(())
}

async fn execute_status(setup: &NoirSetup) -> Result<()> {
    let status = setup.check_status().await;

    println!("=== Noir Prover Status ===\n");

    // BB Binary
    println!("Barretenberg (bb) Binary:");
    println!("  Path: {}", setup.bb_binary.display());
    if setup.bb_binary.exists() {
        println!("  ✓ Installed");
    } else {
        println!("  ✗ Not installed");
    }

    println!();

    // Circuits
    println!("Circuit Artifacts:");
    println!("  Path: {}", setup.circuits_dir.display());
    if setup.circuits_dir.exists() {
        println!("  ✓ Installed");
    } else {
        println!("  ✗ Not installed");
    }

    println!();

    // Overall status
    match status {
        SetupStatus::Ready => {
            println!("Status: ✓ Ready for proof generation");
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
        }
        SetupStatus::FullSetupNeeded => {
            println!("Status: ✗ Full setup required");
            println!("Run `enclave noir setup` to complete installation");
        }
    }

    Ok(())
}

async fn execute_setup(setup: &NoirSetup, force: bool) -> Result<()> {
    if force {
        println!("Force reinstalling Noir prover components...\n");
        // For force reinstall, we'd need to delete and recreate
        // For now, just run ensure_installed which will update if needed
    } else {
        // Check if already set up
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

async fn execute_prove(
    setup: &NoirSetup,
    circuit_name: &str,
    inputs_path: &str,
    e3_id: &str,
) -> Result<()> {
    use e3_noir_prover::Circuit;
    use std::collections::HashMap;

    let circuit = match circuit_name.to_lowercase().as_str() {
        "pk-bfv" | "pkbfv" => Circuit::PkBfv,
        "dec-share-trbfv" | "decsharetrbfv" => Circuit::DecShareTrbfv,
        "verify-shares" | "verifyshares" => Circuit::VerifyShares,
        _ => bail!(
            "Unknown circuit: {}. Available: pk-bfv, dec-share-trbfv, verify-shares",
            circuit_name
        ),
    };

    // Read inputs from TOML file
    let inputs_content = std::fs::read_to_string(inputs_path)
        .with_context(|| format!("Failed to read inputs file: {}", inputs_path))?;

    let inputs: HashMap<String, String> =
        toml::from_str(&inputs_content).with_context(|| "Failed to parse inputs TOML")?;

    println!("Generating proof for circuit: {:?}", circuit);
    println!("Using inputs from: {}", inputs_path);

    let prover = NoirProver::new(setup.clone());
    let proof = prover
        .generate_proof(circuit, &inputs, e3_id)
        .await
        .map_err(|e| anyhow!("Proof generation failed: {}", e))?;

    println!("\n✓ Proof generated successfully!");
    println!("  Proof size: {} bytes", proof.bytes.len());

    Ok(())
}

async fn execute_verify(
    setup: &NoirSetup,
    circuit_name: &str,
    proof_path: &str,
    _e3_id: &str,
) -> Result<()> {
    use e3_noir_prover::Circuit;

    let circuit = match circuit_name.to_lowercase().as_str() {
        "pk-bfv" | "pkbfv" => Circuit::PkBfv,
        "dec-share-trbfv" | "decsharetrbfv" => Circuit::DecShareTrbfv,
        "verify-shares" | "verifyshares" => Circuit::VerifyShares,
        _ => bail!(
            "Unknown circuit: {}. Available: pk-bfv, dec-share-trbfv, verify-shares",
            circuit_name
        ),
    };

    // Read proof from file
    let proof_data = std::fs::read(proof_path)
        .with_context(|| format!("Failed to read proof file: {}", proof_path))?;

    let proof = e3_noir_prover::Proof {
        bytes: proof_data,
        circuit: circuit.clone(),
    };

    println!("Verifying proof for circuit: {:?}", circuit);

    let prover = NoirProver::new(setup.clone());
    prover
        .verify_proof(circuit, &proof)
        .await
        .map_err(|e| anyhow!("Verification failed: {}", e))?;

    println!("\n✓ Proof is valid!");

    Ok(())
}
