// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! On-chain ZK proof verification tests.
//! Requires: `bb`, `anvil`, and compiled contract artifacts
//! (`npx hardhat compile` in packages/enclave-contracts).

mod common;

use alloy::{
    network::TransactionBuilder,
    primitives::{Bytes, FixedBytes},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    sol,
};
use common::{find_anvil, find_bb, setup_compiled_circuit, setup_test_prover};
use e3_fhe_params::BfvPreset;
use e3_zk_helpers::circuits::dkg::pk::circuit::{PkCircuit, PkCircuitData};
use e3_zk_prover::{Provable, ZkProver};
use std::path::PathBuf;

sol! {
    #[sol(rpc)]
    contract DkgPkVerifier {
        function verify(bytes calldata proof, bytes32[] calldata publicInputs) external view returns (bool verified);
    }
}

/// Linker placeholder that gets replaced with the deployed ZKTranscriptLib address.
const ZK_TRANSCRIPT_LIB_PLACEHOLDER: &str = "__$3f925933ac313a1c84f3f4c25b9ea43c90$__";

fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/enclave-contracts/artifacts/contracts/verifier/DkgPkVerifier.sol")
}

fn read_artifact_bytecode_hex(artifact_name: &str) -> Option<String> {
    let path = artifacts_dir().join(artifact_name);
    let json_str = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;
    json["bytecode"].as_str().map(|s| s.to_string())
}

fn decode_bytecode(hex_str: &str) -> Vec<u8> {
    let clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(clean).expect("failed to decode bytecode hex")
}

fn link_transcript_lib(bytecode_hex: &str, lib_address: &alloy::primitives::Address) -> Vec<u8> {
    let addr_hex = hex::encode(lib_address.as_slice());
    let linked = bytecode_hex.replace(ZK_TRANSCRIPT_LIB_PLACEHOLDER, &addr_hex);
    decode_bytecode(&linked)
}

#[tokio::test]
async fn test_pk_bfv_onchain_verification() {
    // Generate ZK proof

    let bb = match find_bb().await {
        Some(bb) => bb,
        None => {
            println!("skipping: bb not found");
            return;
        }
    };

    let preset = BfvPreset::InsecureThreshold512;
    let (backend, _temp) = setup_test_prover(&bb).await;
    setup_compiled_circuit(&backend, "dkg", "pk").await;

    let sample = match PkCircuitData::generate_sample(preset) {
        Ok(s) => s,
        Err(e) => {
            println!("skipping: failed to generate sample: {e}");
            return;
        }
    };

    let prover = ZkProver::new(&backend);
    let e3_id = "0";

    let proof = PkCircuit
        .prove(&prover, &preset, &sample, e3_id)
        .expect("proof generation should succeed");

    assert!(!proof.data.is_empty(), "proof data should not be empty");
    assert!(
        !proof.public_signals.is_empty(),
        "public signals should not be empty"
    );

    let local_ok = PkCircuit.verify(&prover, &proof, e3_id, 1);
    assert!(
        local_ok.as_ref().is_ok_and(|&v| v),
        "local proof verification failed: {local_ok:?}"
    );

    println!(
        "proof: {} bytes, public_inputs: {} bytes",
        proof.data.len(),
        proof.public_signals.len()
    );

    // Deploy verifier contract to Anvil

    let lib_bytecode_hex = match read_artifact_bytecode_hex("ZKTranscriptLib.json") {
        Some(h) => h,
        None => {
            println!(
                "skipping: ZKTranscriptLib artifact not found \
                 (run `npx hardhat compile` in packages/enclave-contracts)"
            );
            return;
        }
    };
    let verifier_bytecode_hex = match read_artifact_bytecode_hex("DkgPkVerifier.json") {
        Some(h) => h,
        None => {
            println!(
                "skipping: DkgPkVerifier artifact not found \
                 (run `npx hardhat compile` in packages/enclave-contracts)"
            );
            return;
        }
    };

    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();

    let lib_bytecode = decode_bytecode(&lib_bytecode_hex);
    let lib_deploy_tx = TransactionRequest::default().with_deploy_code(Bytes::from(lib_bytecode));
    let lib_receipt = provider
        .send_transaction(lib_deploy_tx)
        .await
        .expect("failed to send ZKTranscriptLib deploy tx")
        .get_receipt()
        .await
        .expect("failed to get ZKTranscriptLib deploy receipt");
    let lib_address = lib_receipt
        .contract_address
        .expect("ZKTranscriptLib deploy receipt missing contract address");
    println!("ZKTranscriptLib deployed at: {lib_address}");

    let linked_bytecode = link_transcript_lib(&verifier_bytecode_hex, &lib_address);
    let verifier_deploy_tx =
        TransactionRequest::default().with_deploy_code(Bytes::from(linked_bytecode));
    let verifier_receipt = provider
        .send_transaction(verifier_deploy_tx)
        .await
        .expect("failed to send DkgPkVerifier deploy tx")
        .get_receipt()
        .await
        .expect("failed to get DkgPkVerifier deploy receipt");
    let verifier_address = verifier_receipt
        .contract_address
        .expect("DkgPkVerifier deploy receipt missing contract address");
    println!("DkgPkVerifier deployed at: {verifier_address}");

    let verifier = DkgPkVerifier::new(verifier_address, &provider);

    // Verify proof on-chain

    let proof_bytes = Bytes::copy_from_slice(&proof.data);

    // pk_bfv has 17 public inputs, 16 are pairing points baked into the proof,
    // so only 1 (the pk commitment) gets passed as publicInputs to the contract.
    let public_inputs: Vec<FixedBytes<32>> = proof
        .public_signals
        .chunks(32)
        .map(|chunk| {
            let mut buf = [0u8; 32];
            buf[..chunk.len()].copy_from_slice(chunk);
            FixedBytes::from(buf)
        })
        .collect();

    assert_eq!(
        public_inputs.len(),
        1,
        "pk_bfv circuit should produce exactly 1 public input (commitment), got {}",
        public_inputs.len()
    );

    println!(
        "calling on-chain verify with {} proof bytes, {} public input(s)",
        proof_bytes.len(),
        public_inputs.len()
    );

    let verified = verifier
        .verify(proof_bytes, public_inputs)
        .call()
        .await
        .expect("on-chain verification call reverted — the proof should be valid");

    assert!(
        verified,
        "on-chain ZK proof verification should return true"
    );

    println!("on-chain verification passed");

    prover.cleanup(e3_id).unwrap();
}
