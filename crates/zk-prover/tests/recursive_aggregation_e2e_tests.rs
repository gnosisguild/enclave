// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Tests for `circuits/bin/recursive_aggregation/*` fold binaries under [`CircuitVariant::Default`]
//! (`noir-recursive-no-zk` VK only — see `scripts/build-circuits.ts` for aggregation circuits).
//!
//! - [`recursive_aggregation_default_artifacts_staged`] checks that staged paths match what
//!   [`ZkProver::generate_recursive_aggregation_bin_proof`] / [`e3_zk_prover::ZkProver::verify_fold_proof`]
//!   expect (no `bb prove`).
//! - [`c3_fold_sequential_proves_and_verifies`] runs `bb prove` for two inner `ShareEncryption`
//!   proofs, then [`generate_sequential_c3_fold`] (`c3_fold_kernel` genesis + two `c3_fold` steps)
//!   (requires `pnpm build:circuits` for `share_encryption`, `c3_fold`, and `c3_fold_kernel`).

mod common;

use std::path::PathBuf;

use common::{
    find_bb, setup_compiled_circuit, setup_recursive_aggregation_fold_circuit, setup_test_prover,
};
use e3_events::CircuitName;
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::computation::DkgInputType;
use e3_zk_helpers::dkg::share_encryption::{ShareEncryptionCircuit, ShareEncryptionCircuitData};
use e3_zk_helpers::CiphernodesCommitteeSize;
use e3_zk_prover::{
    generate_sequential_c3_fold, CircuitVariant, CompiledCircuit, Provable, ZkBackend, ZkProver,
};

fn c3_fold_json_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../circuits/bin/recursive_aggregation/c3_fold/target/c3_fold.json")
}

/// Reads `C3_SLOTS` from the compiled `c3_fold` ABI (`acc_public_inputs` length is `4 + 3 * C3_SLOTS`).
fn c3_fold_total_slots_from_compiled_json() -> usize {
    let path = c3_fold_json_path();
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "read {}: {} (run `pnpm build:circuits --group recursive_aggregation`)",
            path.display(),
            e
        )
    });
    let v: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
    let len = v["abi"]["parameters"]
        .as_array()
        .and_then(|ps| {
            ps.iter()
                .find(|p| {
                    p.get("name") == Some(&serde_json::Value::String("acc_public_inputs".into()))
                })
                .and_then(|p| p.get("type")?.get("length")?.as_u64())
        })
        .expect("c3_fold.json: abi.parameters.acc_public_inputs.length") as usize;
    assert!(
        len >= 4 && (len - 4) % 3 == 0,
        "unexpected acc_public_inputs length {} (expected 4 + 3 * slots)",
        len
    );
    (len - 4) / 3
}

#[test]
fn c3_fold_compiled_abi_has_consistent_slot_count() {
    if !c3_fold_json_path().exists() {
        println!(
            "skipping: {} not found (run `pnpm build:circuits --group recursive_aggregation`)",
            c3_fold_json_path().display()
        );
        return;
    }
    let slots = c3_fold_total_slots_from_compiled_json();
    assert!(slots > 0, "C3_SLOTS inferred from ABI should be positive");
    let _ =
        CompiledCircuit::from_file(&c3_fold_json_path()).expect("load compiled c3_fold circuit");
}

#[tokio::test]
async fn recursive_aggregation_default_artifacts_staged() {
    let Some(bb) = find_bb().await else {
        println!("skipping: bb not found");
        return;
    };
    if !c3_fold_json_path().exists() {
        println!("skipping: {} not found", c3_fold_json_path().display());
        return;
    }

    let (backend, temp) = setup_test_prover(&bb).await;
    setup_recursive_aggregation_fold_circuit(&backend, CircuitName::C3Fold).await;

    let base = backend
        .circuits_dir
        .join("insecure-512")
        .join("default")
        .join(CircuitName::C3Fold.dir_path());
    let pkg = CircuitName::C3Fold.as_str();
    assert!(
        base.join(format!("{pkg}.json")).exists(),
        "expected staged {}.json under default/ variant",
        pkg
    );
    assert!(
        base.join(format!("{pkg}.vk")).exists(),
        "expected staged {}.vk (noir-recursive-no-zk) under default/ variant",
        pkg
    );

    drop(temp);
}

async fn setup_c3_fold_with_inner_share_encryption() -> Option<(
    ZkBackend,
    tempfile::TempDir,
    ZkProver,
    ShareEncryptionCircuit,
    ShareEncryptionCircuitData,
    ShareEncryptionCircuitData,
    BfvPreset,
)> {
    let committee = CiphernodesCommitteeSize::Micro.values();
    let preset = BfvPreset::InsecureThreshold512;
    let bb = find_bb().await?;
    let (backend, temp) = setup_test_prover(&bb).await;

    let sd = BfvPreset::InsecureThreshold512.search_defaults()?;

    setup_compiled_circuit(&backend, "dkg", "share_encryption").await;
    setup_recursive_aggregation_fold_circuit(&backend, CircuitName::C3Fold).await;
    setup_recursive_aggregation_fold_circuit(&backend, CircuitName::C3FoldKernel).await;

    let sample_a = ShareEncryptionCircuitData::generate_sample(
        preset,
        committee.clone(),
        DkgInputType::SecretKey,
        sd.z,
        sd.lambda,
    )
    .ok()?;
    let sample_b = ShareEncryptionCircuitData::generate_sample(
        preset,
        committee,
        DkgInputType::SecretKey,
        sd.z,
        sd.lambda,
    )
    .ok()?;
    let prover = ZkProver::new(&backend);

    Some((
        backend,
        temp,
        prover,
        ShareEncryptionCircuit,
        sample_a,
        sample_b,
        preset,
    ))
}

#[tokio::test]
async fn c3_fold_sequential_proves_and_verifies() {
    let Some((_backend, _temp, prover, circuit, sample_a, sample_b, preset)) =
        setup_c3_fold_with_inner_share_encryption().await
    else {
        println!("skipping: bb not found or prerequisites missing");
        return;
    };

    let artifacts_dir = preset.artifacts_dir();
    let inner_e3_a = "e3-c3fold-inner-0";
    let inner_e3_b = "e3-c3fold-inner-1";
    let fold_e3 = "e3-c3fold-step";

    let inner_a = circuit
        .prove_with_variant(
            &prover,
            &preset,
            &sample_a,
            inner_e3_a,
            CircuitVariant::Recursive,
            &artifacts_dir,
        )
        .expect("inner ShareEncryption proof 0");
    assert_eq!(inner_a.circuit, CircuitName::ShareEncryption);

    let inner_b = circuit
        .prove_with_variant(
            &prover,
            &preset,
            &sample_b,
            inner_e3_b,
            CircuitVariant::Recursive,
            &artifacts_dir,
        )
        .expect("inner ShareEncryption proof 1");
    assert_eq!(inner_b.circuit, CircuitName::ShareEncryption);

    let total_slots = c3_fold_total_slots_from_compiled_json();
    assert!(
        total_slots >= 2,
        "need at least 2 C3 slots for two-fold test (compiled total_slots={})",
        total_slots
    );

    let inners = [inner_a, inner_b];
    let folded = generate_sequential_c3_fold(
        &prover,
        &inners,
        &[0u32, 1u32],
        total_slots,
        fold_e3,
        &artifacts_dir,
    )
    .expect("c3_fold sequential fold");
    assert_eq!(folded.circuit, CircuitName::C3Fold);
    assert!(
        !folded.data.is_empty(),
        "fold proof data should not be empty"
    );
    assert!(
        !folded.public_signals.is_empty(),
        "fold public signals should not be empty"
    );

    let party_id = 1u64;
    let ok = prover
        .verify_fold_proof(&folded, fold_e3, party_id, &artifacts_dir)
        .expect("verify_fold_proof invocation");
    assert!(ok, "c3_fold proof should verify under Default VK layout");

    prover.cleanup(inner_e3_a).unwrap();
    prover.cleanup(inner_e3_b).unwrap();
    prover.cleanup(fold_e3).unwrap();
}
