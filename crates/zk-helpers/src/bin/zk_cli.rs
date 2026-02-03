// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! ZK CLI — command-line tool for zero-knowledge circuit artifact generation.
//!
//! This binary lists available circuits and generates Prover.toml and configs.nr
//! for use with the Noir prover. Use `--list_circuits` to see circuits and
//! `--circuit <name> --preset insecure|secure` to generate artifacts.

use anyhow::{anyhow, Context, Result};
use clap::{arg, command, Parser};
use e3_fhe_params::{build_pair_for_preset, BfvPreset};
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitInput};
use e3_zk_helpers::codegen::{write_artifacts, CircuitCodegen};
use e3_zk_helpers::registry::{Circuit, CircuitRegistry};
use e3_zk_helpers::sample::Sample;
use std::path::PathBuf;
use std::sync::Arc;

/// Minimal ZK CLI for generating circuit artifacts.
#[derive(Debug, Parser)]
#[command(name = "zk-cli")]
struct Cli {
    /// List all available circuits and exit.
    #[arg(long)]
    list_circuits: bool,
    /// Circuit name to generate artifacts for (e.g. pk-bfv).
    #[arg(long, required_unless_present = "list_circuits")]
    circuit: Option<String>,
    /// Preset: "insecure" (512) or "secure" (8192). Drives both threshold and DKG params.
    #[arg(long, required_unless_present = "list_circuits")]
    preset: Option<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "output")]
    output: PathBuf,
    /// Skip generating Prover.toml (configs.nr is always generated).
    #[arg(long)]
    toml: bool,
}

/// Security preset: chooses both threshold and DKG params (insecure = 512, secure = 8192).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SecurityPreset {
    Insecure,
    Secure,
}

impl SecurityPreset {
    fn threshold_preset(self) -> BfvPreset {
        match self {
            SecurityPreset::Insecure => BfvPreset::InsecureThreshold512,
            SecurityPreset::Secure => BfvPreset::SecureThreshold8192,
        }
    }
}

/// Parses preset name "insecure" or "secure" into a [`SecurityPreset`].
fn parse_preset(name: &str) -> Result<SecurityPreset> {
    match name.trim() {
        s if s.eq_ignore_ascii_case("insecure") => Ok(SecurityPreset::Insecure),
        s if s.eq_ignore_ascii_case("secure") => Ok(SecurityPreset::Secure),
        _ => Err(anyhow!(
            "unknown preset: {name}. Use \"insecure\" or \"secure\""
        )),
    }
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Register all circuits in the registry (metadata only).
    let mut registry = CircuitRegistry::new();
    registry.register(Arc::new(PkCircuit));

    // Handle list circuits flag.
    if args.list_circuits {
        let circuits = registry.list_circuits();
        println!("Available circuits:");
        for circuit_name in circuits {
            if let Ok(circuit_meta) = registry.get(&circuit_name) {
                println!(
                    "  {} - params_type: {:?}",
                    circuit_name,
                    circuit_meta.supported_parameter(),
                );
            }
        }
        return Ok(());
    }

    // Unwrap required arguments (clap ensures they're present when list_circuits is false).
    let circuit = args.circuit.unwrap();
    let security_preset = parse_preset(&args.preset.unwrap())?;

    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create output dir {}", args.output.display()))?;

    // Validate circuit exists in registry.
    let circuit_meta = registry.get(&circuit).map_err(|_| {
        let available = registry.list_circuits().join(", ");
        anyhow!("unknown circuit: {}. Available: {}", circuit, available)
    })?;

    // Build threshold and DKG params from the security preset (insecure → 512, secure → 8192).
    let (threshold_params, dkg_params) = build_pair_for_preset(security_preset.threshold_preset())
        .map_err(|e| anyhow!("failed to build params: {}", e))?;

    // Validate DKG preset parameter type matches circuit's supported parameter type.
    let dkg_preset = security_preset
        .threshold_preset()
        .dkg_counterpart()
        .expect("threshold preset has DKG counterpart");
    let preset_param_type = dkg_preset.metadata().parameter_type;
    let circuit_param_type = circuit_meta.supported_parameter();
    if preset_param_type != circuit_param_type {
        return Err(anyhow!(
            "preset has parameter type {:?}, but circuit {} requires {:?}",
            preset_param_type,
            circuit,
            circuit_param_type
        ));
    }

    // Generate sample and artifacts based on circuit name from registry.
    let sample = Sample::generate(&threshold_params, &dkg_params, None, 0, 0)?;
    let circuit_name = circuit_meta.name();
    let artifacts = match circuit_name {
        name if name == <PkCircuit as Circuit>::NAME => {
            let circuit = PkCircuit;
            circuit.codegen(
                &dkg_params,
                &PkCircuitInput {
                    public_key: sample.dkg_public_key,
                },
            )?
        }
        name => return Err(anyhow!("circuit {} not yet implemented", name)),
    };

    let toml = if !args.toml {
        None
    } else {
        Some(&artifacts.toml)
    };
    write_artifacts(toml, &artifacts.configs, Some(args.output.as_path()))?;

    println!("Artifacts written to {}", args.output.display());
    Ok(())
}
