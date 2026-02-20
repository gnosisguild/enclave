// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Slashing integration tests: complete flow from proof generation →
//! operator signing → evidence encoding → on-chain SlashingManager verification.
//!
//! ## What these tests prove
//!
//! 1. **Signing format alignment**: Rust `ProofPayload.digest()` produces the
//!    exact structured hash that `SlashingManager.proposeSlash()` reconstructs.
//! 2. **Evidence encoding**: `encode_fault_evidence()` output is correctly
//!    decoded by the Solidity `abi.decode` in `proposeSlash()`.
//! 3. **ECDSA recovery**: Signatures created with alloy's `sign_message_sync`
//!    are correctly recovered on-chain via `ECDSA.recover(toEthSignedMessageHash(...))`.
//! 4. **Complete slashing flow**: Valid proofs revert with `ProofIsValid()`,
//!    wrong signers revert with `SignerIsNotOperator()`, and invalid proofs
//!    result in successful slash execution.
//!
//! ## Prerequisites
//!
//! On-chain tests require:
//! - `anvil` on PATH (from Foundry)
//! - Compiled Hardhat artifacts: `cd packages/enclave-contracts && npx hardhat compile`
//!
//! Run with: `cargo test -p e3-zk-prover --test slashing_integration_tests`

mod common;

use alloy::{
    network::TransactionBuilder,
    primitives::{keccak256, Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    sol,
    sol_types::SolValue,
};
use common::find_anvil;
use e3_events::{
    encode_fault_evidence, CircuitName, E3id, Proof, ProofPayload, ProofType, SignedProofFailed,
    SignedProofPayload,
};
use e3_utils::utility_types::ArcBytes;
use std::path::PathBuf;

// ── Contract ABI definitions (bytecodes loaded from Hardhat artifacts at runtime) ──

sol! {
    #[sol(rpc)]
    contract SlashingManager {
        struct SlashPolicy {
            uint256 ticketPenalty;
            uint256 licensePenalty;
            bool requiresProof;
            address proofVerifier;
            bool banNode;
            uint256 appealWindow;
            bool enabled;
            bool affectsCommittee;
            uint8 failureReason;
        }

        function proposeSlash(uint256 e3Id, address operator, bytes32 reason, bytes calldata proof) external returns (uint256 proposalId);
        function setSlashPolicy(bytes32 reason, SlashPolicy calldata policy) external;
        function totalProposals() external view returns (uint256);
        function isBanned(address node) external view returns (bool);

        error ProofIsValid();
        error SignerIsNotOperator();
        error OperatorNotInCommittee();
        error VerifierMismatch();
    }

    #[sol(rpc)]
    contract MockCircuitVerifier {
        function setReturnValue(bool _returnValue) external;
    }

    #[sol(rpc)]
    contract MockCiphernodeRegistry {
        function setCommitteeNodes(uint256 e3Id, address[] calldata nodes) external;
    }
}

// ── Helpers ──

/// No-op contract deployment bytecode.
///
/// Deploys a contract whose runtime is a single STOP opcode.
/// All calls to this contract succeed with empty return data, making it
/// suitable as a mock for any interface that only has void-returning functions
/// (e.g., IBondingRegistry.slashTicketBalance, IEnclave.onE3Failed).
const NOOP_DEPLOY_BYTECODE: &[u8] = &[
    0x60, 0x01, // PUSH1 0x01 (runtime size)
    0x60, 0x0c, // PUSH1 0x0c (offset of runtime in init code)
    0x60, 0x00, // PUSH1 0x00 (memory destination)
    0x39, //       CODECOPY
    0x60, 0x01, // PUSH1 0x01 (return size)
    0x60, 0x00, // PUSH1 0x00 (return offset)
    0xf3, //       RETURN
    0x00, //       -- runtime: STOP --
];

fn contracts_artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../packages/enclave-contracts/artifacts/contracts")
}

fn read_artifact_bytecode(subpath: &str) -> Option<Vec<u8>> {
    let path = contracts_artifacts_dir().join(subpath);
    let json_str = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&json_str).ok()?;
    let hex_str = json["bytecode"].as_str()?;
    let clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(clean).ok()
}

/// Load all three contract bytecodes, returning None if any are missing.
fn load_slashing_artifacts() -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let sm = read_artifact_bytecode("slashing/SlashingManager.sol/SlashingManager.json")?;
    let mv = read_artifact_bytecode("test/MockSlashingVerifier.sol/MockCircuitVerifier.json")?;
    let mr = read_artifact_bytecode("test/MockCiphernodeRegistry.sol/MockCiphernodeRegistry.json")?;
    Some((sm, mv, mr))
}

/// Deploy a contract on the connected provider.
/// `creation_bytecode` is the compiled init code; `constructor_args` is appended (ABI-encoded).
async fn deploy_contract(
    provider: &impl Provider,
    creation_bytecode: &[u8],
    constructor_args: &[u8],
) -> Address {
    let mut deploy_data = creation_bytecode.to_vec();
    deploy_data.extend_from_slice(constructor_args);
    let tx = TransactionRequest::default().with_deploy_code(Bytes::from(deploy_data));
    let receipt = provider
        .send_transaction(tx)
        .await
        .expect("failed to send deploy tx")
        .get_receipt()
        .await
        .expect("failed to get deploy receipt");
    receipt
        .contract_address
        .expect("deploy receipt missing contract address")
}

/// Create a test ProofPayload with the given parameters.
fn test_proof_payload(e3_id: u64, chain_id: u64) -> ProofPayload {
    ProofPayload {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        proof_type: ProofType::T0PkBfv,
        proof: Proof::new(
            CircuitName::PkBfv,
            ArcBytes::from_bytes(&[0xde, 0xad, 0xbe, 0xef]),
            // One 32-byte public input (padded zero)
            ArcBytes::from_bytes(&[0u8; 32]),
        ),
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Pure Rust tests — no Anvil or artifacts required
// ════════════════════════════════════════════════════════════════════════════

/// Verifies the typehash constant matches the keccak256 of the type string.
#[test]
fn test_proof_payload_typehash() {
    let expected: [u8; 32] = keccak256(
        "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
    )
    .into();
    assert_eq!(
        ProofPayload::typehash(),
        expected,
        "typehash should match keccak256 of the type string"
    );
}

/// Verifies that digest() uses the structured typehash format with hashed dynamic fields.
#[test]
fn test_proof_payload_digest_matches_manual_computation() {
    let payload = test_proof_payload(1, 42);
    let digest = payload.digest().expect("digest should succeed");

    // Manually compute expected digest
    let typehash = keccak256(
        "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
    );
    let expected_encoded = (
        typehash,
        U256::from(42u64),                    // chainId
        U256::from(1u64),                     // e3Id
        U256::from(0u8),                      // proofType (T0PkBfv = 0)
        keccak256(&[0xde, 0xad, 0xbe, 0xef]), // keccak256(zkProof)
        keccak256(&[0u8; 32]),                // keccak256(publicSignals)
    )
        .abi_encode();
    let expected_digest: [u8; 32] = keccak256(&expected_encoded).into();

    assert_eq!(
        digest, expected_digest,
        "digest should match manual computation"
    );
}

/// Verifies sign → recover roundtrip with the structured digest format.
#[test]
fn test_signing_roundtrip_with_structured_digest() {
    let signer: PrivateKeySigner =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse()
            .unwrap();

    let payload = test_proof_payload(42, 31337);
    let signed = SignedProofPayload::sign(payload, &signer).expect("signing should succeed");
    let recovered = signed.recover_address().expect("recovery should succeed");

    assert_eq!(
        recovered,
        signer.address(),
        "recovered address should match signer"
    );
}

/// Verifies that different payloads produce different digests (no collisions).
#[test]
fn test_different_payloads_different_digests() {
    let p1 = test_proof_payload(1, 42);
    let p2 = test_proof_payload(2, 42); // different e3Id
    let mut p3 = test_proof_payload(1, 42);
    p3.proof_type = ProofType::T1PkGeneration; // different proofType

    let d1 = p1.digest().unwrap();
    let d2 = p2.digest().unwrap();
    let d3 = p3.digest().unwrap();

    assert_ne!(d1, d2, "different e3Ids should produce different digests");
    assert_ne!(
        d1, d3,
        "different proofTypes should produce different digests"
    );
}

/// Verifies that encode_fault_evidence() produces correctly structured ABI encoding.
#[test]
fn test_encode_fault_evidence_structure() {
    let signer: PrivateKeySigner =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse()
            .unwrap();
    let verifier_addr: Address = "0x1234567890abcdef1234567890abcdef12345678"
        .parse()
        .unwrap();

    let payload = test_proof_payload(42, 31337);
    let signed = SignedProofPayload::sign(payload, &signer).expect("signing should succeed");

    let failed = SignedProofFailed {
        e3_id: E3id::new("42", 31337),
        faulting_node: signer.address(),
        proof_type: ProofType::T0PkBfv,
        signed_payload: signed.clone(),
    };

    let evidence = encode_fault_evidence(&failed, verifier_addr);

    // Decode and verify structure: (bytes, bytes32[], bytes, uint256, uint256, address)
    type EvidenceTuple = (Bytes, Vec<FixedBytes<32>>, Bytes, U256, U256, Address);
    let decoded = EvidenceTuple::abi_decode_params(&evidence).expect("evidence should ABI-decode");

    let (zk_proof, public_inputs, sig, chain_id, proof_type, verifier) = decoded;

    assert_eq!(&zk_proof[..], &[0xde, 0xad, 0xbe, 0xef], "zkProof mismatch");
    assert_eq!(public_inputs.len(), 1, "should have 1 public input");
    assert_eq!(
        public_inputs[0],
        FixedBytes::from([0u8; 32]),
        "public input value mismatch"
    );
    assert_eq!(&sig[..], &signed.signature[..], "signature bytes mismatch");
    assert_eq!(chain_id, U256::from(31337u64), "chainId mismatch");
    assert_eq!(proof_type, U256::from(0u8), "proofType mismatch");
    assert_eq!(verifier, verifier_addr, "verifier address mismatch");
}

/// Verifies that the digest format matches what Solidity would compute.
///
/// This is the critical cross-language test: if this passes, then:
/// `keccak256(abi.encode(PROOF_PAYLOAD_TYPEHASH, chainId, e3Id, proofType, keccak256(zkProof), keccak256(abi.encodePacked(publicInputs))))`
/// in Solidity produces the same bytes32 as `ProofPayload::digest()` in Rust.
#[test]
fn test_digest_matches_solidity_encoding() {
    let payload = test_proof_payload(42, 31337);
    let digest = payload.digest().expect("digest should succeed");

    // Simulate what Solidity does step by step:
    //
    // bytes32 messageHash = keccak256(abi.encode(
    //     PROOF_PAYLOAD_TYPEHASH,                              // bytes32
    //     chainId,                                              // uint256
    //     e3Id,                                                 // uint256
    //     proofType,                                            // uint256
    //     keccak256(zkProof),                                   // bytes32
    //     keccak256(abi.encodePacked(publicInputs))             // bytes32
    // ));
    //
    // For publicInputs = [bytes32(0)]:
    //   abi.encodePacked(publicInputs) = 0x0000...0000 (32 bytes)
    //   which is the same as the raw publicSignals bytes

    let typehash = keccak256(
        "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
    );

    // abi.encode of all-static types: each word is 32 bytes, no offsets
    let mut solidity_encoded = Vec::with_capacity(192);
    solidity_encoded.extend_from_slice(typehash.as_ref()); // bytes32
    solidity_encoded.extend_from_slice(&U256::from(31337u64).to_be_bytes::<32>()); // uint256 chainId
    solidity_encoded.extend_from_slice(&U256::from(42u64).to_be_bytes::<32>()); // uint256 e3Id
    solidity_encoded.extend_from_slice(&U256::from(0u8).to_be_bytes::<32>()); // uint256 proofType
    solidity_encoded.extend_from_slice(keccak256(&[0xde, 0xad, 0xbe, 0xef]).as_ref()); // keccak256(zkProof)

    // For publicInputs = [bytes32(0)]:
    // Solidity: keccak256(abi.encodePacked(publicInputs)) = keccak256(bytes32(0))
    // Rust: keccak256(public_signals) = keccak256([0u8; 32])
    // These must be the same!
    let sol_public_inputs_hash = keccak256(&[0u8; 32]);
    solidity_encoded.extend_from_slice(sol_public_inputs_hash.as_ref()); // keccak256(publicSignals)

    let solidity_digest: [u8; 32] = keccak256(&solidity_encoded).into();

    assert_eq!(
        digest, solidity_digest,
        "Rust digest must exactly match Solidity messageHash reconstruction"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// On-chain integration tests — require Anvil + compiled Hardhat artifacts
// ════════════════════════════════════════════════════════════════════════════

/// **Complete flow**: operator signs proof → evidence encoded → SlashingManager
/// reconstructs digest, recovers signer, verifies committee membership, and
/// checks ZK proof validity.
///
/// With MockCircuitVerifier returning TRUE (proof is valid), the contract
/// reverts with `ProofIsValid()`. This proves the full Rust→Solidity signing
/// pipeline works correctly.
#[tokio::test]
async fn test_onchain_valid_proof_reverts_proof_is_valid() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mv_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!(
                "skipping: contract artifacts not found \
                 (run `npx hardhat compile` in packages/enclave-contracts)"
            );
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();
    let accounts = provider.get_accounts().await.unwrap();
    let admin = accounts[0];

    // Operator uses a separate key (not an Anvil pre-funded account)
    let operator_signer = PrivateKeySigner::random();
    let operator_addr = operator_signer.address();

    // Deploy infrastructure contracts
    let noop_addr = deploy_contract(&provider, NOOP_DEPLOY_BYTECODE, &[]).await;
    let mock_verifier_addr = deploy_contract(&provider, &mv_bytecode, &[]).await;
    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;

    // Deploy SlashingManager(admin, bondingRegistry, ciphernodeRegistry, enclave)
    let sm_args = (admin, noop_addr, mock_registry_addr, noop_addr).abi_encode();
    let sm_addr = deploy_contract(&provider, &sm_bytecode, &sm_args).await;

    println!("deployed: SlashingManager={sm_addr}, MockVerifier={mock_verifier_addr}, MockRegistry={mock_registry_addr}");
    println!("operator: {operator_addr} (chain_id: {chain_id})");

    // Bind contract instances
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);
    let mock_verifier = MockCircuitVerifier::new(mock_verifier_addr, &provider);
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    // ── Setup: slash policy + committee ──

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: mock_verifier_addr,
                banNode: false,
                appealWindow: U256::ZERO,
                enabled: true,
                affectsCommittee: false,
                failureReason: 0u8,
            },
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    mock_registry
        .setCommitteeNodes(U256::from(e3_id), vec![operator_addr])
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // MockCircuitVerifier returns TRUE → proof is valid → no fault
    mock_verifier
        .setReturnValue(true)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // ── Operator signs proof (Rust-side) ──

    let payload = ProofPayload {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        proof_type: ProofType::T0PkBfv,
        proof: Proof::new(
            CircuitName::PkBfv,
            ArcBytes::from_bytes(&[0xde, 0xad, 0xbe, 0xef]),
            ArcBytes::from_bytes(&[0u8; 32]),
        ),
    };

    let signed =
        SignedProofPayload::sign(payload, &operator_signer).expect("signing should succeed");

    // ── FaultSubmitter encodes evidence (Rust-side) ──

    let failed = SignedProofFailed {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        faulting_node: operator_addr,
        proof_type: ProofType::T0PkBfv,
        signed_payload: signed,
    };

    let evidence = encode_fault_evidence(&failed, mock_verifier_addr);

    // ── Submit to SlashingManager (on-chain) ──

    let result = slashing_mgr
        .proposeSlash(
            U256::from(e3_id),
            operator_addr,
            reason,
            Bytes::from(evidence),
        )
        .call()
        .await;

    // Should revert with ProofIsValid — the proof is valid, so there's no fault
    assert!(
        result.is_err(),
        "should revert because the proof is valid (no fault to slash)"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("ProofIsValid") || err_string.contains("0x5b718c5b"),
        "expected ProofIsValid revert, got: {err_string}"
    );

    println!("PASS: valid proof correctly reverts with ProofIsValid — Rust→Solidity signing alignment verified");
}

/// Tests that a wrong signer (attacker) cannot slash an arbitrary operator.
///
/// The attacker signs the proof with their own key but submits it as evidence
/// against a different operator. The contract should reject because the
/// recovered signer doesn't match the target operator.
#[tokio::test]
async fn test_onchain_wrong_signer_reverts_signer_is_not_operator() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mv_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();
    let accounts = provider.get_accounts().await.unwrap();
    let admin = accounts[0];

    let attacker_signer = PrivateKeySigner::random();
    let victim_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let noop_addr = deploy_contract(&provider, NOOP_DEPLOY_BYTECODE, &[]).await;
    let mock_verifier_addr = deploy_contract(&provider, &mv_bytecode, &[]).await;
    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;

    let sm_args = (admin, noop_addr, mock_registry_addr, noop_addr).abi_encode();
    let sm_addr = deploy_contract(&provider, &sm_bytecode, &sm_args).await;

    let slashing_mgr = SlashingManager::new(sm_addr, &provider);
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: mock_verifier_addr,
                banNode: false,
                appealWindow: U256::ZERO,
                enabled: true,
                affectsCommittee: false,
                failureReason: 0u8,
            },
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Add VICTIM to committee (not the attacker)
    mock_registry
        .setCommitteeNodes(U256::from(e3_id), vec![victim_addr])
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Attacker signs the proof with their own key
    let payload = ProofPayload {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        proof_type: ProofType::T0PkBfv,
        proof: Proof::new(
            CircuitName::PkBfv,
            ArcBytes::from_bytes(&[0xde, 0xad]),
            ArcBytes::from_bytes(&[0u8; 32]),
        ),
    };
    let signed =
        SignedProofPayload::sign(payload, &attacker_signer).expect("signing should succeed");

    let failed = SignedProofFailed {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        faulting_node: attacker_signer.address(),
        proof_type: ProofType::T0PkBfv,
        signed_payload: signed,
    };

    let evidence = encode_fault_evidence(&failed, mock_verifier_addr);

    // Submit evidence targeting the VICTIM, but signed by the ATTACKER
    let result = slashing_mgr
        .proposeSlash(
            U256::from(e3_id),
            victim_addr, // <-- target is victim, not the actual signer
            reason,
            Bytes::from(evidence),
        )
        .call()
        .await;

    assert!(result.is_err(), "should revert because signer != operator");

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("SignerIsNotOperator") || err_string.contains("0xcd659038"),
        "expected SignerIsNotOperator revert, got: {err_string}"
    );

    println!("PASS: wrong signer correctly reverts — V-001 protection verified");
}

/// Tests that operators not in the committee cannot be slashed.
#[tokio::test]
async fn test_onchain_non_committee_member_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mv_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();
    let accounts = provider.get_accounts().await.unwrap();
    let admin = accounts[0];

    let operator_signer = PrivateKeySigner::random();
    let operator_addr = operator_signer.address();

    let noop_addr = deploy_contract(&provider, NOOP_DEPLOY_BYTECODE, &[]).await;
    let mock_verifier_addr = deploy_contract(&provider, &mv_bytecode, &[]).await;
    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;

    let sm_args = (admin, noop_addr, mock_registry_addr, noop_addr).abi_encode();
    let sm_addr = deploy_contract(&provider, &sm_bytecode, &sm_args).await;

    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: mock_verifier_addr,
                banNode: false,
                appealWindow: U256::ZERO,
                enabled: true,
                affectsCommittee: false,
                failureReason: 0u8,
            },
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // NOTE: We do NOT add the operator to the committee

    // Operator signs a proof
    let payload = ProofPayload {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        proof_type: ProofType::T0PkBfv,
        proof: Proof::new(
            CircuitName::PkBfv,
            ArcBytes::from_bytes(&[0xab, 0xcd]),
            ArcBytes::from_bytes(&[0u8; 32]),
        ),
    };
    let signed =
        SignedProofPayload::sign(payload, &operator_signer).expect("signing should succeed");

    let failed = SignedProofFailed {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        faulting_node: operator_addr,
        proof_type: ProofType::T0PkBfv,
        signed_payload: signed,
    };

    let evidence = encode_fault_evidence(&failed, mock_verifier_addr);

    let result = slashing_mgr
        .proposeSlash(
            U256::from(e3_id),
            operator_addr,
            reason,
            Bytes::from(evidence),
        )
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because operator is not in committee"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("OperatorNotInCommittee") || err_string.contains("0x7353fac5"),
        "expected OperatorNotInCommittee revert, got: {err_string}"
    );

    println!("PASS: non-committee member correctly reverts — committee check verified");
}

/// Tests the complete slash execution flow: invalid proof → fault confirmed → slash executed.
///
/// Uses a NOOP contract as BondingRegistry so that `slashTicketBalance` and
/// `slashLicenseBond` calls succeed silently, allowing the full flow to complete.
#[tokio::test]
async fn test_onchain_invalid_proof_executes_slash() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mv_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();
    let accounts = provider.get_accounts().await.unwrap();
    let admin = accounts[0];

    let operator_signer = PrivateKeySigner::random();
    let operator_addr = operator_signer.address();

    let noop_addr = deploy_contract(&provider, NOOP_DEPLOY_BYTECODE, &[]).await;
    let mock_verifier_addr = deploy_contract(&provider, &mv_bytecode, &[]).await;
    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;

    // Use noop as both bondingRegistry and enclave
    let sm_args = (admin, noop_addr, mock_registry_addr, noop_addr).abi_encode();
    let sm_addr = deploy_contract(&provider, &sm_bytecode, &sm_args).await;

    let slashing_mgr = SlashingManager::new(sm_addr, &provider);
    let mock_verifier = MockCircuitVerifier::new(mock_verifier_addr, &provider);
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: mock_verifier_addr,
                banNode: false,
                appealWindow: U256::ZERO,
                enabled: true,
                affectsCommittee: false,
                failureReason: 0u8,
            },
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    mock_registry
        .setCommitteeNodes(U256::from(e3_id), vec![operator_addr])
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // MockCircuitVerifier returns FALSE → proof is invalid → fault confirmed
    mock_verifier
        .setReturnValue(false)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Operator signs a (bad) proof
    let payload = ProofPayload {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        proof_type: ProofType::T0PkBfv,
        proof: Proof::new(
            CircuitName::PkBfv,
            ArcBytes::from_bytes(&[0xba, 0xd0, 0xba, 0xd0]),
            ArcBytes::from_bytes(&[0u8; 32]),
        ),
    };
    let signed =
        SignedProofPayload::sign(payload, &operator_signer).expect("signing should succeed");

    let failed = SignedProofFailed {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        faulting_node: operator_addr,
        proof_type: ProofType::T0PkBfv,
        signed_payload: signed,
    };

    let evidence = encode_fault_evidence(&failed, mock_verifier_addr);

    // Verify proposal count before
    let proposals_before = slashing_mgr
        .totalProposals()
        .call()
        .await
        .expect("totalProposals call failed");
    assert_eq!(
        proposals_before,
        U256::ZERO,
        "should have 0 proposals before"
    );

    // Submit slash — should succeed (invalid proof = fault confirmed)
    let receipt = slashing_mgr
        .proposeSlash(
            U256::from(e3_id),
            operator_addr,
            reason,
            Bytes::from(evidence),
        )
        .send()
        .await
        .expect("proposeSlash tx should not fail to send")
        .get_receipt()
        .await
        .expect("proposeSlash receipt should be obtainable");

    assert!(
        receipt.status(),
        "proposeSlash transaction should succeed (invalid proof = fault confirmed, slash executed)"
    );

    // Verify proposal was created and executed
    let proposals_after = slashing_mgr
        .totalProposals()
        .call()
        .await
        .expect("totalProposals call failed");
    assert_eq!(
        proposals_after,
        U256::from(1u64),
        "should have 1 proposal after slash"
    );

    println!("PASS: invalid proof correctly triggers slash execution — full flow verified");
}

/// Tests that verifier mismatch is detected (verifier-upgrade protection).
///
/// If the evidence references an old verifier address but the policy has been
/// updated to a new verifier, proposeSlash should revert with VerifierMismatch.
#[tokio::test]
async fn test_onchain_verifier_mismatch_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mv_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();
    let accounts = provider.get_accounts().await.unwrap();
    let admin = accounts[0];

    let operator_signer = PrivateKeySigner::random();
    let operator_addr = operator_signer.address();

    let noop_addr = deploy_contract(&provider, NOOP_DEPLOY_BYTECODE, &[]).await;
    let mock_verifier_addr = deploy_contract(&provider, &mv_bytecode, &[]).await;
    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;

    let sm_args = (admin, noop_addr, mock_registry_addr, noop_addr).abi_encode();
    let sm_addr = deploy_contract(&provider, &sm_bytecode, &sm_args).await;

    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: mock_verifier_addr,
                banNode: false,
                appealWindow: U256::ZERO,
                enabled: true,
                affectsCommittee: false,
                failureReason: 0u8,
            },
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    let payload = ProofPayload {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        proof_type: ProofType::T0PkBfv,
        proof: Proof::new(
            CircuitName::PkBfv,
            ArcBytes::from_bytes(&[0xab]),
            ArcBytes::from_bytes(&[0u8; 32]),
        ),
    };
    let signed =
        SignedProofPayload::sign(payload, &operator_signer).expect("signing should succeed");

    let failed = SignedProofFailed {
        e3_id: E3id::new(&e3_id.to_string(), chain_id),
        faulting_node: operator_addr,
        proof_type: ProofType::T0PkBfv,
        signed_payload: signed,
    };

    // Encode evidence pointing to a DIFFERENT verifier (simulating stale evidence)
    let stale_verifier: Address = "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        .parse()
        .unwrap();
    let evidence = encode_fault_evidence(&failed, stale_verifier);

    let result = slashing_mgr
        .proposeSlash(
            U256::from(e3_id),
            operator_addr,
            reason,
            Bytes::from(evidence),
        )
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because verifier in evidence doesn't match policy"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("VerifierMismatch") || err_string.contains("0x1c485278"),
        "expected VerifierMismatch revert, got: {err_string}"
    );

    println!("PASS: verifier mismatch correctly reverts — verifier-upgrade protection verified");
}
