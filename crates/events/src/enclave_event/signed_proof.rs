// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Signed proof payload types for fault attribution.
//!
//! Every ZK proof a node broadcasts is wrapped in a [`SignedProofPayload`] — the node's
//! ECDSA signature over the canonical encoding of the data + proof.  If the proof later
//! fails verification, the signed bundle is self-authenticating evidence of fault:
//! the signature proves authorship and the proof bytes prove invalidity.

use crate::{CircuitName, E3id, Proof};
use actix::Message;
use alloy::primitives::{keccak256, Address, FixedBytes, Signature, U256};
use alloy::signers::{local::PrivateKeySigner, SignerSync};
use alloy::sol_types::SolValue;
use anyhow::{anyhow, Result};
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Proof type identifier covering all node-generated proofs.
///
/// Aggregation proofs (Proofs 5 and 7) are excluded — they are published on-chain
/// directly and verified by the contract at submission time.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofType {
    /// T0 — BFV public key proof (Proof 0).
    T0PkBfv = 0,
    /// T1 — TrBFV public key generation proof (Proof 1).
    T1PkGeneration = 1,
    /// T1 — Secret key share computation proof (Proof 2a).
    T1SkShareComputation = 2,
    /// T1 — Smudging noise share computation proof (Proof 2b).
    T1ESmShareComputation = 3,
    /// T1 — Share encryption proof (Proof 3).
    T1ShareEncryption = 4,
    /// T2 — DKG share decryption proof (Proof 4).
    T2DkgShareDecryption = 5,
    /// T5 — Threshold share decryption proof (Proof 6).
    T5ShareDecryption = 6,
    /// T6 — Decrypted shares aggregation proof (Proof 7).
    T6DecryptedSharesAggregation = 7,
}

impl ProofType {
    /// Map this proof type to its corresponding circuit names.
    pub fn circuit_names(&self) -> Vec<CircuitName> {
        match self {
            ProofType::T0PkBfv => vec![CircuitName::PkBfv],
            ProofType::T1PkGeneration => vec![CircuitName::PkGeneration],
            ProofType::T1SkShareComputation => vec![CircuitName::SkShareComputation],
            ProofType::T1ESmShareComputation => vec![CircuitName::ESmShareComputation],
            ProofType::T1ShareEncryption => vec![CircuitName::ShareEncryption],
            ProofType::T2DkgShareDecryption => vec![CircuitName::DkgShareDecryption],
            ProofType::T5ShareDecryption => vec![CircuitName::ThresholdShareDecryption],
            ProofType::T6DecryptedSharesAggregation => vec![
                CircuitName::DecryptedSharesAggregationBn,
                CircuitName::DecryptedSharesAggregationMod,
            ],
        }
    }

    /// Slash reason identifier for on-chain policies.
    pub fn slash_reason(&self) -> &'static str {
        match self {
            ProofType::T0PkBfv
            | ProofType::T1PkGeneration
            | ProofType::T1SkShareComputation
            | ProofType::T1ESmShareComputation
            | ProofType::T1ShareEncryption
            | ProofType::T2DkgShareDecryption => "E3_BAD_DKG_PROOF",
            ProofType::T5ShareDecryption => "E3_BAD_DECRYPTION_PROOF",
            ProofType::T6DecryptedSharesAggregation => "E3_BAD_AGGREGATION_PROOF",
        }
    }
}

impl fmt::Display for ProofType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Data payload that a node signs before broadcasting.
///
/// Only contains data needed for on-chain fault verification:
/// the E3 identifier, proof type, and the ZK proof itself.
/// Encoded via `abi.encode(chainId, e3Id, proofType, proof, publicSignals)`
/// so on-chain `ecrecover` can reconstruct the same digest.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct ProofPayload {
    /// E3 computation identifier.
    pub e3_id: E3id,
    /// Which proof this payload carries.
    pub proof_type: ProofType,
    /// The ZK proof that attests to the data.
    pub proof: Proof,
}

impl ProofPayload {
    /// The typehash that domain-separates the signed message.
    ///
    /// Must match `PROOF_PAYLOAD_TYPEHASH` in `SlashingManager.sol`:
    /// `keccak256("ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)")`
    pub fn typehash() -> [u8; 32] {
        keccak256(
            "ProofPayload(uint256 chainId,uint256 e3Id,uint256 proofType,bytes zkProof,bytes publicSignals)",
        )
        .into()
    }

    /// Compute the keccak256 digest of the canonical encoding.
    ///
    /// Uses structured hashing with a typehash prefix for domain separation,
    /// and keccak256-hashes the dynamic fields (`zkProof`, `publicSignals`)
    /// for gas efficiency on the Solidity verification side.
    ///
    /// The encoding is:
    /// ```text
    /// keccak256(abi.encode(
    ///     PROOF_PAYLOAD_TYPEHASH,   // bytes32
    ///     chainId,                   // uint256
    ///     e3Id,                      // uint256
    ///     proofType,                 // uint256
    ///     keccak256(zkProof),        // bytes32
    ///     keccak256(publicSignals)   // bytes32
    /// ))
    /// ```
    ///
    /// This matches the reconstruction in `SlashingManager.proposeSlash()`.
    pub fn digest(&self) -> Result<[u8; 32]> {
        let e3_id_u256: U256 = self
            .e3_id
            .clone()
            .try_into()
            .map_err(|_| anyhow!("E3id cannot be converted to U256"))?;

        let typehash = Self::typehash();

        // keccak256(abi.encode(typehash, chainId, e3Id, proofType, keccak256(proof), keccak256(publicSignals)))
        // All fields are bytes32/uint256 → pure static ABI encoding (6 × 32 = 192 bytes)
        let encoded = (
            typehash,
            U256::from(self.e3_id.chain_id()),
            e3_id_u256,
            U256::from(self.proof_type as u8),
            keccak256(&*self.proof.data),
            keccak256(&*self.proof.public_signals),
        )
            .abi_encode();

        Ok(keccak256(&encoded).into())
    }
}

/// Signed wrapper around a [`ProofPayload`].
///
/// This is the unit of data broadcast over the p2p network.  The signature
/// is an Ethereum-style `eth_sign` (EIP-191 personal message) over the
/// keccak256 digest of the payload's canonical encoding.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct SignedProofPayload {
    /// The payload that was signed.
    pub payload: ProofPayload,
    /// 65-byte ECDSA signature (r ‖ s ‖ v) computed via `eth_sign`.
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub signature: ArcBytes,
}

impl SignedProofPayload {
    /// Sign a [`ProofPayload`] with the node's ECDSA key.
    pub fn sign(payload: ProofPayload, signer: &PrivateKeySigner) -> Result<Self> {
        let digest = payload.digest()?;
        let sig = signer
            .sign_message_sync(&digest)
            .map_err(|e| anyhow!("Failed to sign proof payload: {e}"))?;

        Ok(Self {
            payload,
            signature: ArcBytes::from_bytes(&sig.as_bytes()),
        })
    }

    /// Recover the Ethereum address that produced this signature.
    pub fn recover_address(&self) -> Result<Address> {
        let sig = Signature::try_from(&self.signature[..])
            .map_err(|e| anyhow!("Invalid signature: {e}"))?;

        let digest = self.payload.digest()?;
        sig.recover_address_from_msg(&digest)
            .map_err(|e| anyhow!("Failed to recover address: {e}"))
    }

    /// Verify that the recovered address matches the expected address.
    pub fn verify_address(&self, expected: &Address) -> Result<bool> {
        let recovered = self.recover_address()?;
        Ok(recovered == *expected)
    }
}

/// Emitted when a node detects a signed proof that fails ZK verification.
///
/// This event carries the complete evidence bundle: the bad proof bytes,
/// the public signals, and the faulting node's signature.  The
/// [`FaultSubmitter`] actor consumes this to submit a slash proposal
/// on-chain.
#[derive(Message, Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
#[derivative(Debug)]
pub struct SignedProofFailed {
    /// E3 computation identifier.
    pub e3_id: E3id,
    /// Ethereum address of the faulting node (recovered from signature).
    pub faulting_node: Address,
    /// Which proof type failed.
    pub proof_type: ProofType,
    /// The full signed payload — self-authenticating evidence.
    pub signed_payload: SignedProofPayload,
}

impl Display for SignedProofFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SignedProofFailed {{ e3_id: {}, faulting_node: {}, proof_type: {} }}",
            self.e3_id, self.faulting_node, self.proof_type
        )
    }
}

/// Encode a [`SignedProofFailed`] event into the ABI-encoded evidence bytes
/// expected by `SlashingManager.proposeSlash()`.
///
/// Returns: `abi.encode(bytes zkProof, bytes32[] publicInputs, bytes signature, uint256 chainId, uint256 proofType, address verifier)`
///
/// The `verifier` is the current on-chain verifier contract address for this
/// proof type's slash policy. The `FaultSubmitter` actor must look this up
/// before calling this function.
pub fn encode_fault_evidence(failed: &SignedProofFailed, verifier: Address) -> Vec<u8> {
    use alloy::primitives::Bytes;

    let proof = &failed.signed_payload.payload.proof;

    // Convert raw public_signals bytes → Vec<FixedBytes<32>> (one per 32-byte field)
    let public_inputs: Vec<FixedBytes<32>> = proof
        .public_signals
        .chunks(32)
        .map(|chunk| {
            let mut buf = [0u8; 32];
            buf[..chunk.len()].copy_from_slice(chunk);
            FixedBytes::from(buf)
        })
        .collect();

    // Must match the decode in SlashingManager.proposeSlash():
    // (bytes zkProof, bytes32[] publicInputs, bytes signature, uint256 chainId, uint256 proofType, address verifier)
    //
    // IMPORTANT: Use abi_encode_params() (not abi_encode()) because abi_encode()
    // wraps dynamic tuples in an outer offset word, but Solidity's abi.decode()
    // expects flat parameter encoding — the same as abi.encode(a, b, c, ...).
    (
        Bytes::copy_from_slice(&proof.data),
        public_inputs,
        Bytes::copy_from_slice(&failed.signed_payload.signature),
        U256::from(failed.e3_id.chain_id()),
        U256::from(failed.signed_payload.payload.proof_type as u8),
        verifier,
    )
        .abi_encode_params()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signer() -> PrivateKeySigner {
        // Deterministic test key
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse()
            .unwrap()
    }

    fn test_payload() -> ProofPayload {
        ProofPayload {
            e3_id: E3id::new("1", 42),
            proof_type: ProofType::T0PkBfv,
            proof: Proof::new(
                CircuitName::PkBfv,
                ArcBytes::from_bytes(&[10, 20, 30]),
                ArcBytes::from_bytes(&[100, 200]),
            ),
        }
    }

    #[test]
    fn sign_and_recover_roundtrip() {
        let signer = test_signer();
        let payload = test_payload();

        let signed =
            SignedProofPayload::sign(payload.clone(), &signer).expect("signing should succeed");

        let recovered = signed.recover_address().expect("recovery should succeed");
        assert_eq!(recovered, signer.address());
    }

    #[test]
    fn verify_address_correct() {
        let signer = test_signer();
        let payload = test_payload();

        let signed = SignedProofPayload::sign(payload, &signer).expect("signing should succeed");
        assert!(signed
            .verify_address(&signer.address())
            .expect("verify should succeed"));
    }

    #[test]
    fn verify_address_wrong() {
        let signer = test_signer();
        let payload = test_payload();

        let signed = SignedProofPayload::sign(payload, &signer).expect("signing should succeed");

        let wrong_addr: Address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        assert!(!signed
            .verify_address(&wrong_addr)
            .expect("verify should succeed"));
    }

    #[test]
    fn different_payloads_produce_different_digests() {
        let p1 = test_payload();
        let mut p2 = test_payload();
        p2.proof_type = ProofType::T1PkGeneration;

        assert_ne!(
            p1.digest().expect("digest should succeed"),
            p2.digest().expect("digest should succeed")
        );
    }

    #[test]
    fn tampered_payload_fails_recovery() {
        let signer = test_signer();
        let payload = test_payload();

        let mut signed =
            SignedProofPayload::sign(payload, &signer).expect("signing should succeed");
        // Tamper with the payload after signing
        signed.payload.proof_type = ProofType::T1PkGeneration;

        let recovered = signed.recover_address().expect("recovery should succeed");
        // Recovered address won't match the signer because payload was tampered
        assert_ne!(recovered, signer.address());
    }

    #[test]
    fn proof_type_circuit_names_mapping() {
        assert_eq!(ProofType::T0PkBfv.circuit_names(), vec![CircuitName::PkBfv]);
        assert_eq!(
            ProofType::T1PkGeneration.circuit_names(),
            vec![CircuitName::PkGeneration]
        );
        assert_eq!(
            ProofType::T1ShareEncryption.circuit_names(),
            vec![CircuitName::ShareEncryption]
        );
        assert_eq!(
            ProofType::T2DkgShareDecryption.circuit_names(),
            vec![CircuitName::DkgShareDecryption]
        );
        assert_eq!(
            ProofType::T5ShareDecryption.circuit_names(),
            vec![CircuitName::ThresholdShareDecryption]
        );
        assert_eq!(
            ProofType::T6DecryptedSharesAggregation.circuit_names(),
            vec![
                CircuitName::DecryptedSharesAggregationBn,
                CircuitName::DecryptedSharesAggregationMod,
            ]
        );
    }
}
