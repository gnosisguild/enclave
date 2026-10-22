// SPDX-License-Identifier: LGPL-3.0-only

//! Real secure-preset C6/C7 acceptance, separate from the insecure recursive fixture.

mod common;

use e3_committee::CiphernodesCommitteeSize;
use e3_events::CircuitVariant;
use e3_fhe_params::BfvPreset;
use e3_zk_config::threshold::{decrypted_shares_aggregation, share_decryption};
use e3_zk_prover::{
    DecryptedSharesAggregationCircuit, Provable, ThresholdShareDecryptionCircuit, ZkProver,
};

#[tokio::test]
#[ignore = "requires pinned bb and freshly built secure-8192 C6/C7 artifacts for the selected committee"]
async fn secure_synchronized_decryption_leaves_prove_and_verify() {
    let preset = BfvPreset::SecureThreshold8192;
    let committee = std::env::var("E3_SECURE_SYNC_COMMITTEE")
        .unwrap_or_else(|_| "minimum".into())
        .parse::<CiphernodesCommitteeSize>()
        .expect("valid secure sync committee required");
    let active: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            e3_zk_prover::test_utils::workspace_root().join("circuits/bin/.active-preset.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(active["preset"], "secure-8192");
    assert_eq!(active["committee"], committee.as_str());
    let bb = common::find_bb().await.expect("pinned bb required");
    let (backend, _work) = common::setup_test_prover(&bb).await;
    for circuit in ["share_decryption", "decrypted_shares_aggregation"] {
        common::setup_compiled_circuit_for_preset(
            &backend,
            "threshold",
            circuit,
            "secure-8192",
            committee.as_str(),
        )
        .await;
    }
    let prover = ZkProver::new(&backend);
    let directory = preset.artifacts_dir_for_committee(committee.as_str());
    let c7_config = decrypted_shares_aggregation::Configs::for_preset(preset, committee).unwrap();
    let (aggregate, shares) =
        e3_zk_samples::threshold::decrypted_shares_aggregation::generate_correlated(
            &c7_config, preset,
        )
        .unwrap();
    let c6_config = share_decryption::Configs::for_preset(preset).unwrap();
    let mut commitments = Vec::new();
    for (index, share) in shares.iter().enumerate() {
        let label = format!("secure-sync-c6-{index}");
        let proof = ThresholdShareDecryptionCircuit
            .prove(&prover, &c6_config, share, &label, &directory)
            .unwrap();
        assert!(prover
            .verify_proof(&proof, &label, index as u64, &directory)
            .unwrap());
        commitments.push(proof.extract_output("d_commitment").unwrap().to_vec());
        if index == 0 {
            let changed = changed_public_input(&proof);
            assert!(!prover
                .verify_proof(&changed, "secure-sync-c6-tampered", 0, &directory)
                .unwrap());
        }
        export_public_proof(&proof, &label);
        println!("Verified secure C6 share {index} ({})", committee.as_str());
    }
    let proof = DecryptedSharesAggregationCircuit
        .prove_with_variant(
            &prover,
            &c7_config,
            &aggregate,
            "secure-sync-c7",
            CircuitVariant::Default,
            &directory,
        )
        .unwrap();
    assert!(prover
        .verify_proof_with_variant(
            &proof,
            "secure-sync-c7",
            0,
            CircuitVariant::Default,
            &directory
        )
        .unwrap());
    for (index, commitment) in commitments.iter().enumerate() {
        assert_eq!(
            &proof.public_signals[index * 32..(index + 1) * 32],
            commitment
        );
    }
    let changed = changed_public_input(&proof);
    assert!(!prover
        .verify_proof_with_variant(
            &changed,
            "secure-sync-c7-tampered",
            0,
            CircuitVariant::Default,
            &directory,
        )
        .unwrap());
    export_public_proof(&proof, "secure-sync-c7");
    println!(
        "Verified secure C7 and C6 commitment bindings ({})",
        committee.as_str()
    );
}

fn changed_public_input(proof: &e3_events::Proof) -> e3_events::Proof {
    let mut changed = proof.clone();
    let mut signals = proof.public_signals.to_vec();
    let was_zero = signals[..32].iter().all(|&byte| byte == 0);
    signals[..32].fill(0);
    if was_zero {
        signals[31] = 1;
    }
    changed.public_signals = e3_utils::utility_types::ArcBytes::from_bytes(&signals);
    changed
}

fn export_public_proof(proof: &e3_events::Proof, label: &str) {
    let Some(directory) = std::env::var_os("E3_SECURE_SYNC_PUBLIC_PROOFS") else {
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    // Proof carries only the circuit name, proof bytes and public inputs, never the witness.
    std::fs::write(
        directory.join(format!("{label}.json")),
        serde_json::to_vec_pretty(proof).unwrap(),
    )
    .unwrap();
}
