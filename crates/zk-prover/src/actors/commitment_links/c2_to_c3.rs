// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C2a/C2b (ShareComputation) -> C3a/C3b (ShareEncryption) message commitment
//! consistency links.
//!
//! ## Circuit layouts
//!
//! **C2 (ShareComputation)** outputs `(key_hash, commitment)`.  The
//! `commitment` field sits at the TAIL of `public_signals`.
//!
//! **C3 (ShareEncryption)** takes `expected_pk_commitment` and
//! `expected_message_commitment` as public inputs at the HEAD of
//! `public_signals`.
//!
//! ## Check
//!
//! C2's `commitment` output must equal C3's `expected_message_commitment`
//! input.

use super::{CommitmentLink, FieldValue, LinkScope};
use e3_events::{CircuitName, ProofType};
use e3_zk_helpers::FIELD_BYTE_LEN;

/// C2a -> C3a message commitment consistency link.
pub struct C2aToC3aMessageCommitmentLink;

impl CommitmentLink for C2aToC3aMessageCommitmentLink {
    fn name(&self) -> &'static str {
        "C2a->C3a message_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C2aSkShareComputation
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C3aSkShareEncryption
    }

    fn scope(&self) -> LinkScope {
        LinkScope::SameParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        extract_commitment(public_signals)
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        check_message_commitment_consistency(source_values, target_public_signals)
    }
}

/// C2b -> C3b message commitment consistency link.
pub struct C2bToC3bMessageCommitmentLink;

impl CommitmentLink for C2bToC3bMessageCommitmentLink {
    fn name(&self) -> &'static str {
        "C2b->C3b message_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C2bESmShareComputation
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C3bESmShareEncryption
    }

    fn scope(&self) -> LinkScope {
        LinkScope::SameParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        extract_commitment(public_signals)
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        check_message_commitment_consistency(source_values, target_public_signals)
    }
}

/// Extract the `commitment` output from C2 (ShareComputation) public signals.
fn extract_commitment(public_signals: &[u8]) -> Vec<FieldValue> {
    let layout = CircuitName::ShareComputation.output_layout();
    let Some(bytes) = layout.extract_field(public_signals, "commitment") else {
        return vec![];
    };
    let mut value = [0u8; FIELD_BYTE_LEN];
    value.copy_from_slice(bytes);
    vec![value]
}

/// Shared check: source `commitment` must equal C3's
/// `expected_message_commitment` input at the HEAD of target public signals.
fn check_message_commitment_consistency(
    source_values: &[FieldValue],
    target_public_signals: &[u8],
) -> bool {
    if source_values.is_empty() {
        return false;
    }
    let layout = CircuitName::ShareEncryption.input_layout();
    let Some(target_bytes) =
        layout.extract_field(target_public_signals, "expected_message_commitment")
    else {
        return false;
    };
    target_bytes == source_values[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_field(val: u8) -> [u8; 32] {
        let mut f = [0u8; 32];
        f[31] = val;
        f
    }

    /// Build C2 public signals: 1 pub input + 2 outputs (key_hash, commitment).
    /// Simulate 1 pub input field → total 96 bytes.
    fn c2_signals(key_hash: [u8; 32], commitment: [u8; 32]) -> Vec<u8> {
        let mut signals = vec![0xAA; 32]; // 1 pub input
        signals.extend_from_slice(&key_hash);
        signals.extend_from_slice(&commitment);
        signals
    }

    /// Build C3 public signals: 2 inputs at HEAD.
    fn c3_signals(expected_pk: [u8; 32], expected_msg: [u8; 32]) -> Vec<u8> {
        let mut signals = Vec::new();
        signals.extend_from_slice(&expected_pk);
        signals.extend_from_slice(&expected_msg);
        signals
    }

    #[test]
    fn extract_commitment_from_c2() {
        let link = C2aToC3aMessageCommitmentLink;
        let commitment = make_field(42);
        let signals = c2_signals(make_field(10), commitment);
        let values = link.extract_source_values(&signals);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], commitment);
    }

    #[test]
    fn consistency_passes_when_commitment_matches_c3a() {
        let link = C2aToC3aMessageCommitmentLink;
        let commitment = make_field(42);
        let source_values = vec![commitment];
        let target = c3_signals(make_field(99), commitment);
        assert!(link.check_consistency(&source_values, &target));
    }

    #[test]
    fn consistency_fails_when_commitment_mismatches_c3a() {
        let link = C2aToC3aMessageCommitmentLink;
        let commitment = make_field(42);
        let source_values = vec![commitment];
        let target = c3_signals(make_field(99), make_field(55));
        assert!(!link.check_consistency(&source_values, &target));
    }

    #[test]
    fn consistency_passes_when_commitment_matches_c3b() {
        let link = C2bToC3bMessageCommitmentLink;
        let commitment = make_field(7);
        let source_values = vec![commitment];
        let target = c3_signals(make_field(99), commitment);
        assert!(link.check_consistency(&source_values, &target));
    }

    #[test]
    fn consistency_fails_when_commitment_mismatches_c3b() {
        let link = C2bToC3bMessageCommitmentLink;
        let commitment = make_field(7);
        let source_values = vec![commitment];
        let target = c3_signals(make_field(99), make_field(8));
        assert!(!link.check_consistency(&source_values, &target));
    }

    #[test]
    fn empty_source_is_inconsistent() {
        let link = C2aToC3aMessageCommitmentLink;
        // Empty source values means malformed proof — should be inconsistent
        assert!(!link.check_consistency(&[], &[0u8; 64]));
    }

    #[test]
    fn short_target_signals_is_inconsistent() {
        let link = C2aToC3aMessageCommitmentLink;
        assert!(!link.check_consistency(&[make_field(1)], &[0u8; 31]));
    }

    #[test]
    fn short_source_signals_returns_empty() {
        let link = C2aToC3aMessageCommitmentLink;
        // C2 needs at least 64 bytes for 2 outputs; 32 is too short
        assert!(link.extract_source_values(&[0u8; 32]).is_empty());
    }
}
