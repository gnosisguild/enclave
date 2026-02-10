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
use alloy::primitives::{keccak256, Address, Signature};
use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner, SignerSync};
use anyhow::{anyhow, Result};
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Proof type identifier covering all node-generated proofs.
///
/// Aggregation proofs (Proofs 5 and 7) are excluded — they are published on-chain
/// directly and verified by the contract at submission time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofType {
    /// T0 — BFV public key proof (Proof 0).
    T0PkBfv,
    /// T1 — TrBFV public key generation proof (Proof 1).
    T1PkGeneration,
    /// T1 — Secret key share computation proof (Proof 2a).
    T1SkShareComputation,
    /// T1 — Smudging noise share computation proof (Proof 2b).
    T1ESmShareComputation,
    /// T1 — Secret key share encryption proof (Proof 3a).
    T1SkShareEncryption,
    /// T1 — Smudging noise share encryption proof (Proof 3b).
    T1ESmShareEncryption,
    /// T2 — Secret key share decryption proof (Proof 4a).
    T2SkShareDecryption,
    /// T2 — Smudging noise share decryption proof (Proof 4b).
    T2ESmShareDecryption,
    /// T5 — Share decryption proof (Proof 6).
    T5ShareDecryption,
}

impl ProofType {
    /// Map this proof type to its corresponding circuit name.
    pub fn circuit_name(&self) -> CircuitName {
        match self {
            ProofType::T0PkBfv => CircuitName::PkBfv,
            ProofType::T1PkGeneration => CircuitName::PkGeneration,
            ProofType::T1SkShareComputation
            | ProofType::T1ESmShareComputation
            | ProofType::T1SkShareEncryption
            | ProofType::T1ESmShareEncryption => CircuitName::EncShares,
            ProofType::T2SkShareDecryption | ProofType::T2ESmShareDecryption => {
                CircuitName::DecShares
            }
            ProofType::T5ShareDecryption => CircuitName::DecShares,
        }
    }

    /// Slash reason identifier for on-chain policies.
    pub fn slash_reason(&self) -> &'static str {
        match self {
            ProofType::T0PkBfv
            | ProofType::T1PkGeneration
            | ProofType::T1SkShareComputation
            | ProofType::T1ESmShareComputation
            | ProofType::T1SkShareEncryption
            | ProofType::T1ESmShareEncryption
            | ProofType::T2SkShareDecryption
            | ProofType::T2ESmShareDecryption => "E3_BAD_DKG_PROOF",
            ProofType::T5ShareDecryption => "E3_BAD_DECRYPTION_PROOF",
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
/// The canonical encoding matches Solidity's `abi.encode` layout so that
/// on-chain `ecrecover` can reconstruct the same digest.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct ProofPayload {
    /// E3 computation identifier.
    pub e3_id: E3id,
    /// Which proof this payload carries.
    pub proof_type: ProofType,
    /// The sender's party ID within the committee (0-indexed).
    pub party_id: u64,
    /// The actual data being proven (e.g. `pk_bfv` bytes, share bytes).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub data: ArcBytes,
    /// The ZK proof that attests to `data`.
    pub proof: Proof,
}

impl ProofPayload {
    /// Compute the keccak256 digest of the canonical encoding.
    ///
    /// The encoding concatenates all fields as length-prefixed byte arrays
    /// preceded by fixed-size scalars, matching the structure the on-chain
    /// verifier will reconstruct.
    pub fn digest(&self) -> [u8; 32] {
        // Encode: e3_id chain_id (u64) | e3_id id (u64) | proof_type (u8) | party_id (u64)
        //       | len(data) (u32) | data | len(proof) (u32) | proof
        //       | len(public_signals) (u32) | public_signals
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.e3_id.chain_id().to_be_bytes());
        let id_bytes = self.e3_id.e3_id().as_bytes();
        buf.extend_from_slice(&(id_bytes.len() as u32).to_be_bytes());
        buf.extend_from_slice(id_bytes);
        buf.push(self.proof_type as u8);
        buf.extend_from_slice(&self.party_id.to_be_bytes());
        // data
        buf.extend_from_slice(&(self.data.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.data);
        // proof bytes
        buf.extend_from_slice(&(self.proof.data.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.proof.data);
        // public_signals
        buf.extend_from_slice(&(self.proof.public_signals.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.proof.public_signals);

        keccak256(&buf).into()
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
    pub fn sign(payload: ProofPayload, signer: &LocalSigner<SigningKey>) -> Result<Self> {
        let digest = payload.digest();
        let sig: Signature = signer
            .sign_message_sync(&digest)
            .map_err(|e| anyhow!("Failed to sign proof payload: {e}"))?;

        // Encode as 65-byte r ‖ s ‖ v
        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&sig.r().to_be_bytes::<32>());
        sig_bytes.extend_from_slice(&sig.s().to_be_bytes::<32>());
        sig_bytes.push(sig.v() as u8);

        Ok(Self {
            payload,
            signature: ArcBytes::from_bytes(&sig_bytes),
        })
    }

    /// Recover the Ethereum address that produced this signature.
    pub fn recover_signer(&self) -> Result<Address> {
        if self.signature.len() != 65 {
            return Err(anyhow!(
                "Invalid signature length: expected 65, got {}",
                self.signature.len()
            ));
        }

        let r = alloy::primitives::U256::from_be_slice(&self.signature[..32]);
        let s = alloy::primitives::U256::from_be_slice(&self.signature[32..64]);
        let v = self.signature[64] != 0;
        let sig = Signature::new(r, s, v);

        let digest = self.payload.digest();
        // EIP-191 personal message hash: "\x19Ethereum Signed Message:\n32" ++ digest
        let prefixed = alloy::primitives::eip191_hash_message(digest);

        sig.recover_address_from_prehash(&prefixed)
            .map_err(|e| anyhow!("Failed to recover signer address: {e}"))
    }

    /// Verify that the recovered signer matches the expected address.
    pub fn verify_signer(&self, expected: &Address) -> Result<bool> {
        let recovered = self.recover_signer()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::local::PrivateKeySigner;

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
            party_id: 3,
            data: ArcBytes::from_bytes(&[1, 2, 3, 4]),
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

        let recovered = signed.recover_signer().expect("recovery should succeed");
        assert_eq!(recovered, signer.address());
    }

    #[test]
    fn verify_signer_correct_address() {
        let signer = test_signer();
        let payload = test_payload();

        let signed = SignedProofPayload::sign(payload, &signer).expect("signing should succeed");
        assert!(signed
            .verify_signer(&signer.address())
            .expect("verify should succeed"));
    }

    #[test]
    fn verify_signer_wrong_address() {
        let signer = test_signer();
        let payload = test_payload();

        let signed = SignedProofPayload::sign(payload, &signer).expect("signing should succeed");

        let wrong_addr: Address = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        assert!(!signed
            .verify_signer(&wrong_addr)
            .expect("verify should succeed"));
    }

    #[test]
    fn different_payloads_produce_different_digests() {
        let p1 = test_payload();
        let mut p2 = test_payload();
        p2.party_id = 99;

        assert_ne!(p1.digest(), p2.digest());
    }

    #[test]
    fn tampered_payload_fails_recovery() {
        let signer = test_signer();
        let payload = test_payload();

        let mut signed =
            SignedProofPayload::sign(payload, &signer).expect("signing should succeed");
        // Tamper with the payload after signing
        signed.payload.party_id = 999;

        let recovered = signed.recover_signer().expect("recovery should succeed");
        // Recovered address won't match the signer because payload was tampered
        assert_ne!(recovered, signer.address());
    }

    #[test]
    fn proof_type_circuit_name_mapping() {
        assert_eq!(ProofType::T0PkBfv.circuit_name(), CircuitName::PkBfv);
        assert_eq!(
            ProofType::T1PkGeneration.circuit_name(),
            CircuitName::PkGeneration
        );
        assert_eq!(
            ProofType::T1SkShareEncryption.circuit_name(),
            CircuitName::EncShares
        );
        assert_eq!(
            ProofType::T2SkShareDecryption.circuit_name(),
            CircuitName::DecShares
        );
    }
}
