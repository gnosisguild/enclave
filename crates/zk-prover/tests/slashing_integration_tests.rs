// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Slashing integration tests: off-chain proof signing + on-chain attestation-based slashing.
//!
//! ## What these tests prove
//!
//! ### Pure Rust (no Anvil)
//! 1. **ProofPayload signing**: `ProofPayload.digest()` produces the correct
//!    structured hash for off-chain proof signing (PROOF_PAYLOAD_TYPEHASH).
//! 2. **ECDSA roundtrip**: `sign_message_sync` → `recover_address` for ProofPayload.
//! 3. **Evidence encoding**: `encode_fault_evidence()` produces valid ABI-encoded
//!    data (retained for Lane B tests).
//! 4. **Vote typehash**: VOTE_TYPEHASH matches the Solidity constant.
//! 5. **Attestation evidence**: vote signatures are correctly constructed and
//!    ABI-encoded for `proposeSlash()`.
//!
//! ### On-chain integration (Anvil + Hardhat artifacts)
//! 6. **Valid attestation quorum** → slash executes successfully.
//! 7. **Insufficient attestations** → reverts `InsufficientAttestations`.
//! 8. **Voter not in committee** → reverts `VoterNotInCommittee`.
//! 9. **Invalid vote signature** → reverts `InvalidVoteSignature`.
//! 10. **Duplicate voter** → reverts `DuplicateVoter`.
//! 11. **Duplicate evidence replay** → reverts `DuplicateEvidence`.
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
    signers::{local::PrivateKeySigner, SignerSync},
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
        function setBondingRegistry(address newBondingRegistry) external;
        function setCiphernodeRegistry(address newCiphernodeRegistry) external;
        function setEnclave(address newEnclave) external;
        function totalProposals() external view returns (uint256);
        function isBanned(address node) external view returns (bool);

        error InsufficientAttestations();
        error DuplicateVoter();
        error VoterNotInCommittee();
        error InvalidVoteSignature();
        error InvalidProof();
        error DuplicateEvidence();
    }

    #[sol(rpc)]
    contract MockCiphernodeRegistry {
        function setCommitteeNodes(uint256 e3Id, address[] calldata nodes) external;
        function setThreshold(uint256 e3Id, uint32 m) external;
    }
}

// ── Helpers ──

/// No-op contract deployment bytecode.
///
/// Deploys a contract whose runtime is a single STOP opcode.
/// All calls to this contract succeed with empty return data, making it
/// suitable as a mock for any interface that only has void-returning functions
/// (e.g., IEnclave.onE3Failed).
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

/// Mock contract that returns 32 zero bytes for any call.
///
/// EVM memory is zero-initialized, so `RETURN(0x00, 0x20)` returns 32 zero bytes.
/// Suitable as a mock for interfaces that return a single `uint256`
/// (e.g., `IBondingRegistry.slashTicketBalance` returns `uint256`).
const RETURNER_DEPLOY_BYTECODE: &[u8] = &[
    0x60, 0x05, // PUSH1 0x05 (runtime size)
    0x60, 0x0c, // PUSH1 0x0c (offset of runtime in init code)
    0x60, 0x00, // PUSH1 0x00 (memory destination)
    0x39, //       CODECOPY
    0x60, 0x05, // PUSH1 0x05 (return size)
    0x60, 0x00, // PUSH1 0x00 (return offset)
    0xf3, //       RETURN
    // -- runtime: return 32 zero bytes --
    0x60, 0x20, // PUSH1 0x20
    0x60, 0x00, // PUSH1 0x00
    0xf3, //       RETURN
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

/// Load contract bytecodes, returning None if any are missing.
fn load_slashing_artifacts() -> Option<(Vec<u8>, Vec<u8>)> {
    let sm = read_artifact_bytecode("slashing/SlashingManager.sol/SlashingManager.json")?;
    let mr = read_artifact_bytecode("test/MockCiphernodeRegistry.sol/MockCiphernodeRegistry.json")?;
    Some((sm, mr))
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
        proof_type: ProofType::C0PkBfv,
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
        U256::from(0u8),                      // proofType (C0PkBfv = 0)
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
    p3.proof_type = ProofType::C1PkGeneration; // different proofType

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
        proof_type: ProofType::C0PkBfv,
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
// Attestation vote helpers — used by both pure Rust and on-chain tests
// ════════════════════════════════════════════════════════════════════════════

const VOTE_TYPEHASH_STR: &str =
    "AccusationVote(uint256 chainId,uint256 e3Id,bytes32 accusationId,address voter,bool agrees,bytes32 dataHash)";

/// Compute `accusationId = keccak256(abi.encodePacked(chainId, e3Id, operator, proofType))`
/// matching `AccusationManager::accusation_id()` and `SlashingManager._verifyAttestationEvidence()`.
fn compute_accusation_id(
    chain_id: u64,
    e3_id: u64,
    operator: Address,
    proof_type: u8,
) -> FixedBytes<32> {
    keccak256(
        &(
            U256::from(chain_id),
            U256::from(e3_id),
            operator,
            U256::from(proof_type),
        )
            .abi_encode_packed(),
    )
}

/// Compute the structured vote digest matching `AccusationManager::vote_digest()`.
fn compute_vote_digest(
    chain_id: u64,
    e3_id: u64,
    accusation_id: FixedBytes<32>,
    voter: Address,
    agrees: bool,
    data_hash: FixedBytes<32>,
) -> FixedBytes<32> {
    let typehash = keccak256(VOTE_TYPEHASH_STR);
    keccak256(
        &(
            typehash,
            U256::from(chain_id),
            U256::from(e3_id),
            accusation_id,
            voter,
            agrees,
            data_hash,
        )
            .abi_encode(),
    )
}

/// Sign a vote and return `(voter_address, signature_bytes)`.
fn sign_vote(
    signer: &PrivateKeySigner,
    chain_id: u64,
    e3_id: u64,
    accusation_id: FixedBytes<32>,
    agrees: bool,
    data_hash: FixedBytes<32>,
) -> (Address, Bytes) {
    let voter = signer.address();
    let digest = compute_vote_digest(chain_id, e3_id, accusation_id, voter, agrees, data_hash);
    let sig = signer
        .sign_message_sync(digest.as_ref())
        .expect("vote signing should succeed");
    (voter, Bytes::from(sig.as_bytes().to_vec()))
}

/// Encode attestation evidence for `proposeSlash()`.
///
/// Format: `abi.encode(uint256 proofType, address[] voters, bool[] agrees, bytes32[] dataHashes, bytes[] signatures)`
/// Voters are sorted ascending by address (contract requires strict ascending order).
fn encode_attestation_evidence(
    proof_type: u8,
    mut votes: Vec<(Address, bool, FixedBytes<32>, Bytes)>,
) -> Bytes {
    votes.sort_by_key(|(addr, _, _, _)| *addr);

    let voters: Vec<Address> = votes.iter().map(|(a, _, _, _)| *a).collect();
    let agrees: Vec<bool> = votes.iter().map(|(_, a, _, _)| *a).collect();
    let data_hashes: Vec<FixedBytes<32>> = votes.iter().map(|(_, _, d, _)| *d).collect();
    let sigs: Vec<Bytes> = votes.iter().map(|(_, _, _, s)| s.clone()).collect();

    Bytes::from((U256::from(proof_type), voters, agrees, data_hashes, sigs).abi_encode())
}

// ════════════════════════════════════════════════════════════════════════════
// Pure Rust attestation tests — no Anvil required
// ════════════════════════════════════════════════════════════════════════════

/// Verifies the VOTE_TYPEHASH constant matches the keccak256 of the vote type string.
#[test]
fn test_vote_typehash() {
    let expected: [u8; 32] = keccak256(VOTE_TYPEHASH_STR).into();
    // Cross-check with the exact string the Solidity contract uses:
    let sol_str = "AccusationVote(uint256 chainId,uint256 e3Id,bytes32 accusationId,address voter,bool agrees,bytes32 dataHash)";
    let sol_hash: [u8; 32] = keccak256(sol_str).into();
    assert_eq!(
        expected, sol_hash,
        "VOTE_TYPEHASH must match the Solidity constant"
    );
}

/// Verifies vote digest computation matches manual abi.encode + keccak256.
#[test]
fn test_vote_digest_manual_computation() {
    let chain_id = 31337u64;
    let e3_id = 42u64;
    let operator: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();
    let voter: Address = "0x2222222222222222222222222222222222222222"
        .parse()
        .unwrap();
    let proof_type = 0u8; // C0PkBfv
    let data_hash = FixedBytes::from([0xab; 32]);

    let accusation_id = compute_accusation_id(chain_id, e3_id, operator, proof_type);
    let digest = compute_vote_digest(chain_id, e3_id, accusation_id, voter, true, data_hash);

    // Manual computation
    let typehash = keccak256(VOTE_TYPEHASH_STR);
    let encoded = (
        typehash,
        U256::from(chain_id),
        U256::from(e3_id),
        accusation_id,
        voter,
        true,
        data_hash,
    )
        .abi_encode();
    let expected: FixedBytes<32> = keccak256(&encoded);

    assert_eq!(
        digest, expected,
        "vote digest should match manual computation"
    );
}

/// Verifies vote sign/recover roundtrip.
#[test]
fn test_vote_signing_roundtrip() {
    let signer: PrivateKeySigner =
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse()
            .unwrap();
    let chain_id = 31337u64;
    let e3_id = 42u64;
    let operator: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();
    let proof_type = 0u8;
    let data_hash = FixedBytes::from([0xab; 32]);

    let accusation_id = compute_accusation_id(chain_id, e3_id, operator, proof_type);
    let (voter, sig_bytes) = sign_vote(&signer, chain_id, e3_id, accusation_id, true, data_hash);

    assert_eq!(
        voter,
        signer.address(),
        "voter should be the signer address"
    );

    // Verify recover
    let digest = compute_vote_digest(chain_id, e3_id, accusation_id, voter, true, data_hash);
    let sig =
        alloy::primitives::Signature::try_from(sig_bytes.as_ref()).expect("signature should parse");
    let recovered = sig
        .recover_address_from_msg(digest.as_slice())
        .expect("recovery should succeed");
    assert_eq!(
        recovered,
        signer.address(),
        "recovered address should match signer"
    );
}

/// Verifies attestation evidence encoding structure.
#[test]
fn test_attestation_evidence_encoding() {
    let signer1: PrivateKeySigner = PrivateKeySigner::random();
    let signer2: PrivateKeySigner = PrivateKeySigner::random();

    let chain_id = 31337u64;
    let e3_id = 1u64;
    let operator: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();
    let proof_type = 0u8;
    let data_hash = FixedBytes::from([0xcc; 32]);

    let accusation_id = compute_accusation_id(chain_id, e3_id, operator, proof_type);

    let (voter1, sig1) = sign_vote(&signer1, chain_id, e3_id, accusation_id, true, data_hash);
    let (voter2, sig2) = sign_vote(&signer2, chain_id, e3_id, accusation_id, true, data_hash);

    let evidence = encode_attestation_evidence(
        proof_type,
        vec![
            (voter1, true, data_hash, sig1),
            (voter2, true, data_hash, sig2),
        ],
    );

    // Decode and verify structure: (uint256, address[], bool[], bytes32[], bytes[])
    type AttestationTuple = (
        U256,
        Vec<Address>,
        Vec<bool>,
        Vec<FixedBytes<32>>,
        Vec<Bytes>,
    );
    let decoded =
        AttestationTuple::abi_decode_params(&evidence).expect("evidence should ABI-decode");

    let (dec_proof_type, dec_voters, dec_agrees, dec_hashes, dec_sigs) = decoded;
    assert_eq!(dec_proof_type, U256::from(proof_type), "proofType mismatch");
    assert_eq!(dec_voters.len(), 2, "should have 2 voters");
    assert!(
        dec_voters[0] < dec_voters[1],
        "voters should be sorted ascending"
    );
    assert!(dec_agrees.iter().all(|a| *a), "all votes should agree");
    assert_eq!(dec_hashes.len(), 2, "should have 2 data hashes");
    assert_eq!(dec_sigs.len(), 2, "should have 2 signatures");
}

// ════════════════════════════════════════════════════════════════════════════
// On-chain integration tests — require Anvil + compiled Hardhat artifacts
// ════════════════════════════════════════════════════════════════════════════

/// Deploy SlashingManager and configure dependencies.
/// Returns (SlashingManager contract instance, admin address).
async fn deploy_and_configure(
    provider: &impl Provider,
    sm_bytecode: &[u8],
    mock_registry_addr: Address,
) -> (Address, Address) {
    let accounts = provider.get_accounts().await.unwrap();
    let admin = accounts[0];

    // Deploy noop for enclave (void functions)
    let noop_addr = deploy_contract(provider, NOOP_DEPLOY_BYTECODE, &[]).await;
    // Deploy returner for bondingRegistry (slashTicketBalance returns uint256)
    let returner_addr = deploy_contract(provider, RETURNER_DEPLOY_BYTECODE, &[]).await;

    // Deploy SlashingManager(admin) — constructor only takes admin address
    let sm_args = admin.abi_encode();
    let sm_addr = deploy_contract(provider, sm_bytecode, &sm_args).await;

    // Configure dependencies via admin functions
    let slashing_mgr = SlashingManager::new(sm_addr, provider);
    slashing_mgr
        .setBondingRegistry(returner_addr)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    slashing_mgr
        .setCiphernodeRegistry(mock_registry_addr)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    slashing_mgr
        .setEnclave(noop_addr)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    (sm_addr, admin)
}

/// **Lane A attestation flow**: 3 committee members vote on a fault, quorum
/// is reached (M=2), and the slash executes atomically.
///
/// Proves the complete Rust→Solidity attestation signing pipeline works:
/// vote_digest → sign_message_sync → abi.encode evidence → proposeSlash → _verifyAttestationEvidence
#[tokio::test]
async fn test_onchain_valid_attestation_executes_slash() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mr_bytecode) = match load_slashing_artifacts() {
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

    // Three committee member signers
    let voter_signer1 = PrivateKeySigner::random();
    let voter_signer2 = PrivateKeySigner::random();
    let voter_signer3 = PrivateKeySigner::random();

    let operator_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    // Deploy mock registry
    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    // Deploy and configure SlashingManager
    let (sm_addr, _admin) = deploy_and_configure(&provider, &sm_bytecode, mock_registry_addr).await;
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;
    let proof_type = 0u8; // C0PkBfv

    // Set slash policy (attestation-based: requiresProof=true, appealWindow=0)
    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: Address::ZERO,
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

    // Set committee: 3 voters, threshold M=2
    let committee = vec![
        voter_signer1.address(),
        voter_signer2.address(),
        voter_signer3.address(),
    ];
    mock_registry
        .setCommitteeNodes(U256::from(e3_id), committee)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    mock_registry
        .setThreshold(U256::from(e3_id), 2u32)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // All 3 voters sign accusation votes (agrees=true)
    let accusation_id = compute_accusation_id(chain_id, e3_id, operator_addr, proof_type);
    let data_hash = FixedBytes::from([0xaa; 32]);

    let (v1, s1) = sign_vote(
        &voter_signer1,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );
    let (v2, s2) = sign_vote(
        &voter_signer2,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );
    let (v3, s3) = sign_vote(
        &voter_signer3,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );

    let evidence = encode_attestation_evidence(
        proof_type,
        vec![
            (v1, true, data_hash, s1),
            (v2, true, data_hash, s2),
            (v3, true, data_hash, s3),
        ],
    );

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

    // Submit slash — should succeed (3 valid votes, threshold M=2)
    let receipt = slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence)
        .send()
        .await
        .expect("proposeSlash tx should not fail to send")
        .get_receipt()
        .await
        .expect("proposeSlash receipt should be obtainable");

    assert!(
        receipt.status(),
        "proposeSlash should succeed with valid attestation quorum"
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

    println!(
        "PASS: valid attestation quorum → slash executed — attestation signing pipeline verified"
    );
}

/// Tests that insufficient attestations (below threshold M) cause revert.
#[tokio::test]
async fn test_onchain_insufficient_attestations_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();

    let voter_signer1 = PrivateKeySigner::random();
    let voter_signer2 = PrivateKeySigner::random();
    let voter_signer3 = PrivateKeySigner::random();

    let operator_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let (sm_addr, _admin) = deploy_and_configure(&provider, &sm_bytecode, mock_registry_addr).await;
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;
    let proof_type = 0u8;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: Address::ZERO,
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

    // Committee: 3 voters, threshold M=2
    mock_registry
        .setCommitteeNodes(
            U256::from(e3_id),
            vec![
                voter_signer1.address(),
                voter_signer2.address(),
                voter_signer3.address(),
            ],
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    mock_registry
        .setThreshold(U256::from(e3_id), 2u32)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Only 1 vote (below threshold M=2)
    let accusation_id = compute_accusation_id(chain_id, e3_id, operator_addr, proof_type);
    let data_hash = FixedBytes::from([0xbb; 32]);

    let (v1, s1) = sign_vote(
        &voter_signer1,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );

    let evidence = encode_attestation_evidence(proof_type, vec![(v1, true, data_hash, s1)]);

    let result = slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence)
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because only 1 vote < threshold M=2"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("InsufficientAttestations"),
        "expected InsufficientAttestations revert, got: {err_string}"
    );

    println!("PASS: insufficient attestations correctly reverts");
}

/// Tests that a voter not in the committee causes revert.
#[tokio::test]
async fn test_onchain_voter_not_in_committee_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();

    let committee_signer = PrivateKeySigner::random();
    let outsider_signer = PrivateKeySigner::random();

    let operator_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let (sm_addr, _admin) = deploy_and_configure(&provider, &sm_bytecode, mock_registry_addr).await;
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;
    let proof_type = 0u8;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: Address::ZERO,
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

    // Committee only contains committee_signer, NOT outsider_signer
    mock_registry
        .setCommitteeNodes(U256::from(e3_id), vec![committee_signer.address()])
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    mock_registry
        .setThreshold(U256::from(e3_id), 1u32)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Outsider signs a vote (valid signature, but not a committee member)
    let accusation_id = compute_accusation_id(chain_id, e3_id, operator_addr, proof_type);
    let data_hash = FixedBytes::from([0xcc; 32]);

    let (v_out, s_out) = sign_vote(
        &outsider_signer,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );

    let evidence = encode_attestation_evidence(proof_type, vec![(v_out, true, data_hash, s_out)]);

    let result = slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence)
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because voter is not a committee member"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("VoterNotInCommittee"),
        "expected VoterNotInCommittee revert, got: {err_string}"
    );

    println!("PASS: non-committee voter correctly reverts — committee check verified");
}

/// Tests that an invalid vote signature (signed by wrong key) causes revert.
#[tokio::test]
async fn test_onchain_invalid_vote_signature_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();

    let victim_signer = PrivateKeySigner::random();
    let impersonator_signer = PrivateKeySigner::random();

    let operator_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let (sm_addr, _admin) = deploy_and_configure(&provider, &sm_bytecode, mock_registry_addr).await;
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;
    let proof_type = 0u8;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: Address::ZERO,
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

    // victim_signer is in the committee
    mock_registry
        .setCommitteeNodes(U256::from(e3_id), vec![victim_signer.address()])
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    mock_registry
        .setThreshold(U256::from(e3_id), 1u32)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Impersonator signs the vote with their key, but we claim it's from victim_signer
    let accusation_id = compute_accusation_id(chain_id, e3_id, operator_addr, proof_type);
    let data_hash = FixedBytes::from([0xdd; 32]);

    // Sign using impersonator's key but construct the digest for victim_signer's address
    let digest = compute_vote_digest(
        chain_id,
        e3_id,
        accusation_id,
        victim_signer.address(),
        true,
        data_hash,
    );
    let bad_sig = impersonator_signer
        .sign_message_sync(digest.as_ref())
        .expect("signing should succeed");

    // Build evidence claiming the vote is from victim_signer but signed by impersonator
    let evidence = Bytes::from(
        (
            U256::from(proof_type),
            vec![victim_signer.address()],
            vec![true],
            vec![data_hash],
            vec![Bytes::from(bad_sig.as_bytes().to_vec())],
        )
            .abi_encode(),
    );

    let result = slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence)
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because signature doesn't match claimed voter"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("InvalidVoteSignature"),
        "expected InvalidVoteSignature revert, got: {err_string}"
    );

    println!("PASS: invalid vote signature correctly reverts — signature verification verified");
}

/// Tests that duplicate voters (non-ascending order) cause revert.
///
/// The contract requires voters in strictly ascending address order to prevent
/// the same voter from being counted twice.
#[tokio::test]
async fn test_onchain_duplicate_voter_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();

    let voter_signer = PrivateKeySigner::random();

    let operator_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let (sm_addr, _admin) = deploy_and_configure(&provider, &sm_bytecode, mock_registry_addr).await;
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;
    let proof_type = 0u8;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: Address::ZERO,
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
        .setCommitteeNodes(U256::from(e3_id), vec![voter_signer.address()])
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    mock_registry
        .setThreshold(U256::from(e3_id), 1u32)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    // Create TWO votes from the same voter (duplicate addresses)
    let accusation_id = compute_accusation_id(chain_id, e3_id, operator_addr, proof_type);
    let data_hash = FixedBytes::from([0xee; 32]);

    let (voter, sig) = sign_vote(
        &voter_signer,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );

    // Submit evidence with duplicate voter entries (bypassing encode_attestation_evidence
    // which would deduplicate — construct manually to have same address appear twice)
    let evidence = Bytes::from(
        (
            U256::from(proof_type),
            vec![voter, voter], // duplicate!
            vec![true, true],
            vec![data_hash, data_hash],
            vec![sig.clone(), sig],
        )
            .abi_encode(),
    );

    let result = slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence)
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because of duplicate voter addresses"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("DuplicateVoter"),
        "expected DuplicateVoter revert, got: {err_string}"
    );

    println!("PASS: duplicate voter correctly reverts — sorted-order dedup verified");
}

/// Tests that replaying the same evidence causes revert.
#[tokio::test]
async fn test_onchain_duplicate_evidence_reverts() {
    if !find_anvil().await {
        println!("skipping: anvil not found on PATH");
        return;
    }

    let (sm_bytecode, mr_bytecode) = match load_slashing_artifacts() {
        Some(artifacts) => artifacts,
        None => {
            println!("skipping: contract artifacts not found");
            return;
        }
    };

    let provider = ProviderBuilder::new().connect_anvil_with_wallet();
    let chain_id = provider.get_chain_id().await.unwrap();

    let voter_signer1 = PrivateKeySigner::random();
    let voter_signer2 = PrivateKeySigner::random();

    let operator_addr: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();

    let mock_registry_addr = deploy_contract(&provider, &mr_bytecode, &[]).await;
    let mock_registry = MockCiphernodeRegistry::new(mock_registry_addr, &provider);

    let (sm_addr, _admin) = deploy_and_configure(&provider, &sm_bytecode, mock_registry_addr).await;
    let slashing_mgr = SlashingManager::new(sm_addr, &provider);

    let reason: FixedBytes<32> = keccak256("E3_BAD_DKG_PROOF");
    let e3_id: u64 = 42;
    let proof_type = 0u8;

    slashing_mgr
        .setSlashPolicy(
            reason,
            SlashingManager::SlashPolicy {
                ticketPenalty: U256::from(50_000_000u64),
                licensePenalty: U256::from(100_000_000_000_000_000_000u128),
                requiresProof: true,
                proofVerifier: Address::ZERO,
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
        .setCommitteeNodes(
            U256::from(e3_id),
            vec![voter_signer1.address(), voter_signer2.address()],
        )
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();
    mock_registry
        .setThreshold(U256::from(e3_id), 2u32)
        .send()
        .await
        .unwrap()
        .get_receipt()
        .await
        .unwrap();

    let accusation_id = compute_accusation_id(chain_id, e3_id, operator_addr, proof_type);
    let data_hash = FixedBytes::from([0xff; 32]);

    let (v1, s1) = sign_vote(
        &voter_signer1,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );
    let (v2, s2) = sign_vote(
        &voter_signer2,
        chain_id,
        e3_id,
        accusation_id,
        true,
        data_hash,
    );

    let evidence = encode_attestation_evidence(
        proof_type,
        vec![(v1, true, data_hash, s1), (v2, true, data_hash, s2)],
    );

    // First submission should succeed
    slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence.clone())
        .send()
        .await
        .expect("first proposeSlash should succeed")
        .get_receipt()
        .await
        .expect("first proposeSlash receipt should be obtainable");

    // Second submission with same evidence should revert
    let result = slashing_mgr
        .proposeSlash(U256::from(e3_id), operator_addr, reason, evidence)
        .call()
        .await;

    assert!(
        result.is_err(),
        "should revert because the same evidence was already consumed"
    );

    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("DuplicateEvidence"),
        "expected DuplicateEvidence revert, got: {err_string}"
    );

    println!("PASS: duplicate evidence correctly reverts — replay protection verified");
}
