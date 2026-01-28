// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use e3_fhe_params::{BfvParamSet, BfvPreset};
use e3_pvss::circuits::pk_bfv::circuit::{PkBfvCircuit, PkBfvCodegenInput};
use e3_pvss::circuits::pk_bfv::codegen::write_artifacts;
use e3_pvss::registry::CircuitRegistry;
use e3_pvss::sample;
use e3_pvss::traits::Circuit;
use e3_pvss::traits::CircuitCodegen;
use std::path::PathBuf;
use std::sync::Arc;

/// Minimal PVSS CLI for generating circuit artifacts.
#[derive(Debug, Parser)]
#[command(name = "pvss-cli")]
struct Cli {
    /// List all available circuits and exit.
    #[arg(long)]
    list_circuits: bool,
    /// Circuit name to generate artifacts for (e.g. pk-bfv).
    #[arg(long, required_unless_present = "list_circuits")]
    circuit: Option<String>,
    /// BFV preset name (must match circuit's parameter type).
    #[arg(long, required_unless_present = "list_circuits")]
    preset: Option<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "output")]
    output: PathBuf,
}

/// Parse a preset name into a BFV preset.
fn parse_preset(name: &str) -> Result<BfvPreset> {
    BfvPreset::from_name(name).map_err(|_| {
        let available = BfvPreset::list().join(", ");
        anyhow!("unknown preset: {name}. Available: {available}")
    })
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Register all circuits in the registry (metadata only).
    let mut registry = CircuitRegistry::new();
    registry.register(Arc::new(PkBfvCircuit));

    // Handle list circuits flag.
    if args.list_circuits {
        let circuits = registry.list_circuits();
        println!("Available circuits:");
        for circuit_name in circuits {
            if let Ok(circuit_meta) = registry.get(&circuit_name) {
                println!(
                    "  {} - params_type: {:?}, n_recursive_proofs: {}, pub_inputs: {}",
                    circuit_name,
                    circuit_meta.supported_parameter(),
                    circuit_meta.n_recursive_proofs(),
                    circuit_meta.n_public_inputs()
                );
            }
        }
        return Ok(());
    }

    // Unwrap required arguments (clap ensures they're present when list_circuits is false).
    let circuit = args.circuit.unwrap();
    let preset = parse_preset(&args.preset.unwrap())?;

    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create output dir {}", args.output.display()))?;

    // Validate circuit exists in registry.
    let circuit_meta = registry.get(&circuit).map_err(|_| {
        let available = registry.list_circuits().join(", ");
        anyhow!("unknown circuit: {}. Available: {}", circuit, available)
    })?;

    // Validate preset parameter type matches circuit's supported parameter type.
    let preset_param_type = preset.metadata().parameter_type;
    let circuit_param_type = circuit_meta.supported_parameter();
    if preset_param_type != circuit_param_type {
        return Err(anyhow!(
            "preset has parameter type {:?}, but circuit {} requires {:?}",
            preset_param_type,
            circuit,
            circuit_param_type
        ));
    }

    // Generate artifacts based on circuit name from registry.
    let params = BfvParamSet::from(preset).build_arc();
    let sample = sample::generate_sample(&params);
    let circuit_name = circuit_meta.name();
    let artifacts = match circuit_name {
        name if name == <PkBfvCircuit as Circuit>::NAME => {
            let circuit = PkBfvCircuit;
            circuit.codegen(PkBfvCodegenInput {
                preset,
                public_key: sample.public_key,
            })?
        }
        name => return Err(anyhow!("circuit {} not yet implemented", name)),
    };

    write_artifacts(
        &artifacts.toml,
        &artifacts.template,
        &artifacts.configs,
        &artifacts.wrapper,
        Some(args.output.as_path()),
    )?;

    println!("Artifacts written to {}", args.output.display());
    Ok(())
}
