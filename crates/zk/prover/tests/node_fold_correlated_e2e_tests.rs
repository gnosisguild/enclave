// SPDX-License-Identifier: LGPL-3.0-only

//! Real correlated C0-C7 pipeline, including non-prefix dealers and directed PRF bindings.

mod common;
#[path = "common/node_fold_witness.rs"]
mod node_fold_witness;

use alloy::primitives::Address;
use e3_committee::CiphernodesCommitteeSize;
use e3_events::{CircuitName, CircuitVariant, Proof};
use e3_fhe_params::BfvPreset;
use e3_zk_config::{dkg, threshold};
use e3_zk_prover::{
    prove_decryption_aggregation_jobs, prove_dkg_aggregation, prove_node_dkg_fold,
    DecryptedSharesAggregationCircuit, DecryptionAggregationJob, DkgAggregationInput,
    DkgShareDecryptionCircuit, NodeDkgFoldInput, PkAggregationCircuit, PkCircuit,
    PkGenerationCircuit, Provable, ShareComputationCircuit, ShareEncryptionCircuit,
    ThresholdShareDecryptionCircuit, ZkProver,
};

#[tokio::test]
#[ignore = "requires freshly generated insecure-64/minimum leaf and recursive artifacts, pinned bb"]
async fn synchronized_non_prefix_dkg_and_decryption_verify_end_to_end() {
    let preset = BfvPreset::InsecureThreshold64;
    let committee = CiphernodesCommitteeSize::Minimum;
    let bb = common::find_bb().await.expect("pinned bb required");
    common::require_minimum_circuits().expect("compile minimum artifacts first");
    let (backend, _work) = common::setup_test_prover(&bb).await;
    for (group, circuit) in [
        ("dkg", "pk"),
        ("dkg", "sk_share_computation"),
        ("dkg", "share_encryption"),
        ("dkg", "share_decryption"),
        ("threshold", "pk_generation"),
        ("threshold", "pk_aggregation"),
        ("threshold", "share_decryption"),
        ("threshold", "decrypted_shares_aggregation"),
    ] {
        common::setup_compiled_circuit(&backend, group, circuit).await;
    }
    for circuit in [
        CircuitName::C3Fold,
        CircuitName::C3FoldKernel,
        CircuitName::NodeFold,
        CircuitName::NodesFold,
        CircuitName::NodesFoldKernel,
        CircuitName::DkgAggregator,
        CircuitName::C6Fold,
        CircuitName::C6FoldKernel,
        CircuitName::DecryptionAggregator,
    ] {
        common::setup_recursive_aggregation_fold_circuit(&backend, circuit).await;
    }
    let prover = ZkProver::new(&backend);
    let dir = preset.artifacts_dir_for_committee(committee.as_str());
    let material = node_fold_witness::generate(preset, committee);
    assert_eq!(material.ids, [0, 2]);
    let mut folds = Vec::new();
    for (dealer, &id) in material.ids.iter().enumerate() {
        let label = format!("sync-dealer-{id}");
        let c0 = PkCircuit
            .prove(
                &prover,
                &dkg::pk::Configs::for_preset(preset).unwrap(),
                &material.c0[dealer],
                &format!("{label}-c0"),
                &dir,
            )
            .unwrap();
        let c1 = PkGenerationCircuit
            .prove(
                &prover,
                &threshold::pk_generation::Configs::for_preset(preset, committee).unwrap(),
                &material.c1[dealer],
                &format!("{label}-c1"),
                &dir,
            )
            .unwrap();
        let c2 = ShareComputationCircuit
            .prove(
                &prover,
                &dkg::share_computation::Configs::for_preset(preset, committee).unwrap(),
                &material.c2[dealer],
                &format!("{label}-c2"),
                &dir,
            )
            .unwrap();
        let mut c3 = Vec::new();
        let mut slots = Vec::new();
        for (slot, data) in &material.c3[dealer] {
            c3.push(
                ShareEncryptionCircuit
                    .prove(
                        &prover,
                        &dkg::share_encryption::Configs::for_preset(preset).unwrap(),
                        data,
                        &format!("{label}-c3-{slot}"),
                        &dir,
                    )
                    .unwrap(),
            );
            slots.push(*slot);
        }
        let c4 = DkgShareDecryptionCircuit
            .prove(
                &prover,
                &dkg::share_decryption::Configs::for_preset(preset).unwrap(),
                &material.c4[dealer],
                &format!("{label}-c4"),
                &dir,
            )
            .unwrap();
        let node = prove_node_dkg_fold(
            &prover,
            &NodeDkgFoldInput {
                c0_proof: &c0,
                c1_proof: &c1,
                c2a_proof: &c2,
                c3a_inner_proofs: &c3,
                c3_slot_indices_a: &slots,
                c3_total_slots: committee.values().n * material.c2[dealer].secret.limbs.len(),
                c4a_proof: &c4,
                party_id: id,
            },
            &label,
            &dir,
        )
        .unwrap()
        .proof;
        assert!(prover
            .verify_proof_with_variant(
                &node,
                &format!("{label}-verify"),
                id,
                CircuitVariant::Default,
                &dir
            )
            .unwrap());
        folds.push(node);
    }
    let c5 = PkAggregationCircuit
        .prove_with_variant(
            &prover,
            &threshold::pk_aggregation::Configs::for_preset(preset, committee).unwrap(),
            &material.c5,
            "sync-c5",
            CircuitVariant::Default,
            &dir,
        )
        .unwrap();
    let addresses = [
        Address::repeat_byte(1),
        Address::repeat_byte(2),
        Address::repeat_byte(3),
    ];
    let dkg = prove_dkg_aggregation(
        &prover,
        &DkgAggregationInput {
            node_fold_proofs: &folds,
            nodes_fold_proof: None,
            c5_proof: &c5,
            party_ids: &material.ids,
            committee_addresses: &addresses,
        },
        "sync-final-dkg",
        preset,
        committee,
    )
    .unwrap();
    assert!(prover
        .verify_proof_with_variant(&dkg, "sync-final-dkg-verify", 0, CircuitVariant::Evm, &dir)
        .unwrap());
    let (c6_data, c7_data) = material.decryption();
    let mut partials = Vec::new();
    for (i, data) in c6_data.iter().enumerate() {
        partials.push(
            ThresholdShareDecryptionCircuit
                .prove(
                    &prover,
                    &threshold::share_decryption::Configs::for_preset(preset).unwrap(),
                    data,
                    &format!("sync-c6-{i}"),
                    &dir,
                )
                .unwrap(),
        );
    }
    let c7 = DecryptedSharesAggregationCircuit
        .prove_with_variant(
            &prover,
            &threshold::decrypted_shares_aggregation::Configs::for_preset(preset, committee)
                .unwrap(),
            &c7_data,
            "sync-c7",
            CircuitVariant::Default,
            &dir,
        )
        .unwrap();
    let slots: Vec<u32> = (0..partials.len() as u32).collect();
    let finals = prove_decryption_aggregation_jobs(
        &prover,
        partials.len(),
        &[DecryptionAggregationJob {
            c6_inner_proofs: &partials,
            c6_slot_indices: &slots,
            c7_proof: &c7,
        }],
        &addresses,
        "sync-final-decryption",
        preset,
        committee,
    )
    .unwrap();
    assert_eq!(finals.len(), 1);
    assert!(prover
        .verify_proof_with_variant(
            &finals[0],
            "sync-final-decryption-verify",
            0,
            CircuitVariant::Evm,
            &dir
        )
        .unwrap());
    let mut forged = finals[0].clone();
    let mut public = forged.public_signals.to_vec();
    let last = public.len() - 1;
    public[last] ^= 1;
    forged.public_signals = e3_utils::ArcBytes::from_bytes(&public);
    assert!(!prover
        .verify_proof_with_variant(&forged, "sync-tampered-key", 0, CircuitVariant::Evm, &dir)
        .unwrap());
    if let Some(path) = std::env::var_os("E3_SYNC_PROOF_OUTPUT") {
        // Export public proofs only, never the correlated secret shares or encryption randomness.
        fn public_proof(proof: &Proof) -> serde_json::Value {
            serde_json::json!({"proof": format!("0x{}", hex::encode(&proof.data[..])),
                "publicInputs": proof.public_signals.chunks_exact(32).map(|word| format!("0x{}", hex::encode(word))).collect::<Vec<_>>()})
        }
        fn folded_proof(proof: &Proof) -> serde_json::Value {
            serde_json::json!({"proof_hex": format!("0x{}", hex::encode(&proof.data[..])),
                "public_inputs_hex": format!("0x{}", hex::encode(&proof.public_signals[..]))})
        }
        std::fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({
            "dkg": public_proof(&dkg), "decryption": public_proof(&finals[0]),
            "committee": addresses.iter().map(|address| address.to_string()).collect::<Vec<_>>(),
            "folded_artifacts": {"dkg_aggregator": folded_proof(&dkg), "decryption_aggregator": folded_proof(&finals[0])},
        })).unwrap()).unwrap();
    }
}
