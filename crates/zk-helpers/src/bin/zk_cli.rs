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
use e3_fhe_params::{build_pair_for_preset, BfvPreset, ParameterType};
use e3_zk_helpers::ciphernodes_committee::CiphernodesCommitteeSize;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitInput};
use e3_zk_helpers::circuits::dkg::share_computation::circuit::{
    ShareComputationCircuit, ShareComputationCircuitInput,
};
use e3_zk_helpers::codegen::{write_artifacts, CircuitCodegen};
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::registry::{Circuit, CircuitRegistry};
use e3_zk_helpers::sample::Sample;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// DKG input type for share-computation circuit: secret key or smudging noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DkgInputTypeArg {
    SecretKey,
    SmudgingNoise,
}

fn parse_input_type(s: &str) -> Result<DkgInputTypeArg> {
    match s.trim().to_lowercase().as_str() {
        "secret-key" => Ok(DkgInputTypeArg::SecretKey),
        "smudging-noise" => Ok(DkgInputTypeArg::SmudgingNoise),
        _ => Err(anyhow!(
            "unknown input-type: {s}. Use \"secret-key\" or \"smudging-noise\""
        )),
    }
}

/// Clear the terminal screen (ANSI escape codes; works on Unix, macOS, and most Windows terminals).
fn clear_terminal() {
    print!("\x1b[2J\x1b[1H");
    let _ = std::io::stdout().flush();
}

/// Print a summary of what will be generated (circuit, preset, witness, output, artifacts).
fn print_generation_info(
    circuit: &str,
    preset: BfvPreset,
    has_witness: bool,
    dkg_input_type: DkgInputType,
    output: &std::path::Path,
    write_prover_toml: bool,
) {
    let meta = preset.metadata();
    println!("  Circuit:  {}", circuit);
    println!(
        "  Preset:   {} (degree {}, {} moduli)",
        preset.security_config_name(),
        meta.degree,
        meta.num_moduli
    );
    if has_witness {
        println!(
            "  Witness:  {}",
            match dkg_input_type {
                DkgInputType::SecretKey => "secret-key",
                DkgInputType::SmudgingNoise => "smudging-noise",
            }
        );
    }
    println!("  Output:   {}", output.display());
    println!("  Artifacts:");
    if write_prover_toml {
        println!("    • configs.nr");
        println!("    • Prover.toml");
    } else {
        println!("    • configs.nr only (pass --toml to also generate Prover.toml)");
    }
    println!();
}

/// Run a closure while showing a spinner. Returns the closure's result.
fn run_with_spinner<F, T, E>(f: F) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E>,
{
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = Arc::clone(&done);
    let spinner = thread::spawn(move || {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0usize;
        while !done_clone.load(Ordering::Relaxed) {
            print!("\r  {} Generating artifacts... ", frames[i % frames.len()]);
            i = i.wrapping_add(1);
            std::io::stdout().flush().ok();
            thread::sleep(Duration::from_millis(80));
        }
    });

    let result = f();
    done.store(true, Ordering::Relaxed);
    spinner.join().ok();
    result
}

/// Print the final success message.
fn print_success(output: &std::path::Path) {
    println!("\r  ✓ Artifacts written to {}", output.display());
}

/// Minimal ZK CLI for generating circuit artifacts.
#[derive(Debug, Parser)]
#[command(name = "zk-cli")]
struct Cli {
    /// List all available circuits and exit.
    #[arg(long)]
    list_circuits: bool,
    /// Circuit name to generate artifacts for (e.g. pk, share-computation).
    #[arg(long, required_unless_present = "list_circuits")]
    circuit: Option<String>,
    /// Preset: "insecure" (512) or "secure" (8192). Drives both threshold and DKG params.
    #[arg(long, required_unless_present = "list_circuits")]
    preset: Option<String>,
    /// For share-computation: witness type "secret-key" or "smudging-noise". Required when circuit is share-computation.
    #[arg(long)]
    witness: Option<String>,
    /// Output directory for generated artifacts.
    #[arg(long, default_value = "output")]
    output: PathBuf,
    /// Also write Prover.toml (default: configs.nr only).
    #[arg(long, default_value = "false")]
    toml: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // Register all circuits in the registry (metadata only).
    let mut registry = CircuitRegistry::new();
    registry.register(Arc::new(PkCircuit));
    registry.register(Arc::new(ShareComputationCircuit));

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
    let preset = BfvPreset::from_security_config_name(&args.preset.unwrap())
        .map_err(|e| anyhow!("{}", e))?;

    std::fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create output dir {}", args.output.display()))?;

    // Validate circuit exists in registry.
    let circuit_meta = registry.get(&circuit).map_err(|_| {
        let available = registry.list_circuits().join(", ");
        anyhow!("unknown circuit: {}. Available: {}", circuit, available)
    })?;

    // Build threshold and DKG params from the preset (insecure → 512, secure → 8192).
    let (threshold_params, dkg_params) =
        build_pair_for_preset(preset).map_err(|e| anyhow!("failed to build params: {}", e))?;

    // Validate preset matches circuit's supported parameter type (THRESHOLD or DKG).
    let circuit_param_type = circuit_meta.supported_parameter();
    let preset_ok = match circuit_param_type {
        ParameterType::THRESHOLD => preset.metadata().parameter_type == ParameterType::THRESHOLD,
        ParameterType::DKG => preset
            .dkg_counterpart()
            .is_some_and(|dkg| dkg.metadata().parameter_type == ParameterType::DKG),
    };
    if !preset_ok {
        return Err(anyhow!(
            "preset does not match circuit {} which requires {:?} (use insecure/secure for threshold circuits)",
            circuit,
            circuit_param_type
        ));
    }

    let write_prover_toml = args.toml;
    // Circuits that accept runtime witness type (e.g. share-computation with SecretKey or SmudgingNoise) have DKG_INPUT_TYPE == None but still need witness handling.
    let has_witness_type = circuit_meta.dkg_input_type().is_some()
        || circuit_meta.name() == ShareComputationCircuit::NAME;

    // For share-computation: require --witness only when generating Prover.toml (configs are shared).
    let dkg_input_type = if has_witness_type {
        let witness_str = if !args.toml {
            // Configs-only: witness optional (configs.nr is the same for both witness types).
            args.witness.as_deref().unwrap_or("secret-key")
        } else {
            // Generating Prover.toml: witness type is required.
            args.witness.as_deref().ok_or_else(|| {
                anyhow!(
                    "circuit {} requires --witness (secret-key or smudging-noise) when writing Prover.toml",
                    circuit
                )
            })?
        };
        let arg = parse_input_type(witness_str)?;
        match arg {
            DkgInputTypeArg::SecretKey => DkgInputType::SecretKey,
            DkgInputTypeArg::SmudgingNoise => DkgInputType::SmudgingNoise,
        }
    } else {
        DkgInputType::SecretKey
    };

    clear_terminal();
    print_generation_info(
        &circuit,
        preset,
        has_witness_type,
        dkg_input_type.clone(),
        &args.output,
        write_prover_toml,
    );

    run_with_spinner(|| {
        let sample = Sample::generate(
            &threshold_params,
            &dkg_params,
            Some(dkg_input_type.clone()),
            CiphernodesCommitteeSize::Small,
            preset.search_defaults().unwrap().z,
            preset.search_defaults().unwrap().lambda,
        )?;
        let circuit_name = circuit_meta.name();
        let artifacts = match circuit_name {
            name if name == <PkCircuit as Circuit>::NAME => {
                let circuit = PkCircuit;
                circuit.codegen(
                    preset,
                    &PkCircuitInput {
                        public_key: sample.dkg_public_key,
                    },
                )?
            }
            name if name == <ShareComputationCircuit as Circuit>::NAME => {
                let circuit = ShareComputationCircuit;
                circuit.codegen(
                    preset,
                    &ShareComputationCircuitInput {
                        dkg_input_type,
                        secret: sample.secret.as_ref().unwrap().clone(),
                        secret_sss: sample.secret_sss.clone(),
                        parity_matrix: sample
                            .parity_matrix
                            .iter()
                            .map(|m| m.to_bigint_rows())
                            .collect(),
                        n_parties: sample.committee.n as u32,
                        threshold: sample.committee.threshold as u32,
                    },
                )?
            }
            name => return Err(anyhow!("circuit {} not yet implemented", name)),
        };

        let toml = if write_prover_toml {
            Some(&artifacts.toml)
        } else {
            None
        };
        write_artifacts(toml, &artifacts.configs, Some(args.output.as_path()))?;
        Ok(())
    })?;

    print_success(&args.output);
    Ok(())
}
