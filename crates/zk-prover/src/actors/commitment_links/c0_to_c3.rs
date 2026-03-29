// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C0 (PkBfv) -> C3a/C3b (ShareEncryption) pk_commitment consistency links.
//!
//! ## Circuit layouts
//!
//! **C0 (PkBfv)** outputs `(pk_commitment)`.  Public signals contain the
//! output at the TAIL.
//!
//! **C3 (ShareEncryption)** takes `expected_pk_commitment` and
//! `expected_message_commitment` as public inputs at the HEAD of
//! `public_signals`.
//!
//! ## Check
//!
//! C0's `pk_commitment` output must equal C3's `expected_pk_commitment` input.

use super::{CommitmentLink, FieldValue, LinkScope};
use e3_events::{CircuitName, ProofType};
use e3_zk_helpers::FIELD_BYTE_LEN;

/// C0 -> C3a pk_commitment consistency link.
pub struct C0ToC3aPkCommitmentLink;

impl CommitmentLink for C0ToC3aPkCommitmentLink {
    fn name(&self) -> &'static str {
        "C0->C3a pk_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C0PkBfv
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C3aSkShareEncryption
    }

    /// Cross-party: node i's C3 encrypts a share under node j's individual
    /// public key, so C0 (source) comes from the recipient while C3 (target)
    /// comes from the sender.
    fn scope(&self) -> LinkScope {
        LinkScope::CrossParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        let layout = CircuitName::PkBfv.output_layout();
        let Some(bytes) = layout.extract_field(public_signals, "pk_commitment") else {
            return vec![];
        };
        let mut value = [0u8; FIELD_BYTE_LEN];
        value.copy_from_slice(bytes);
        vec![value]
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        check_pk_commitment_consistency(source_values, target_public_signals)
    }
}

/// C0 -> C3b pk_commitment consistency link.
pub struct C0ToC3bPkCommitmentLink;

impl CommitmentLink for C0ToC3bPkCommitmentLink {
    fn name(&self) -> &'static str {
        "C0->C3b pk_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C0PkBfv
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C3bESmShareEncryption
    }

    /// Cross-party: same reasoning as C0→C3a — recipient's C0 pk vs sender's C3.
    fn scope(&self) -> LinkScope {
        LinkScope::CrossParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        let layout = CircuitName::PkBfv.output_layout();
        let Some(bytes) = layout.extract_field(public_signals, "pk_commitment") else {
            return vec![];
        };
        let mut value = [0u8; FIELD_BYTE_LEN];
        value.copy_from_slice(bytes);
        vec![value]
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        check_pk_commitment_consistency(source_values, target_public_signals)
    }
}

/// Shared check: source `pk_commitment` must equal C3's `expected_pk_commitment`
/// input at the HEAD of target public signals.
fn check_pk_commitment_consistency(
    source_values: &[FieldValue],
    target_public_signals: &[u8],
) -> bool {
    if source_values.is_empty() {
        return false;
    }
    let layout = CircuitName::ShareEncryption.input_layout();
    let Some(target_bytes) = layout.extract_field(target_public_signals, "expected_pk_commitment")
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

    /// Build C0 public signals: just 1 output (pk_commitment).
    fn c0_signals(pk_commitment: [u8; 32]) -> Vec<u8> {
        pk_commitment.to_vec()
    }

    /// Build C3 public signals: 2 inputs at HEAD + optional extra bytes.
    fn c3_signals(expected_pk: [u8; 32], expected_msg: [u8; 32]) -> Vec<u8> {
        let mut signals = Vec::new();
        signals.extend_from_slice(&expected_pk);
        signals.extend_from_slice(&expected_msg);
        signals
    }

    #[test]
    fn extract_pk_commitment_from_c0() {
        let link = C0ToC3aPkCommitmentLink;
        let pk = make_field(42);
        let signals = c0_signals(pk);
        let values = link.extract_source_values(&signals);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], pk);
    }

    #[test]
    fn consistency_passes_when_pk_matches_c3a() {
        let link = C0ToC3aPkCommitmentLink;
        let pk = make_field(42);
        let source_values = vec![pk];
        let target = c3_signals(pk, make_field(99));
        assert!(link.check_consistency(&source_values, &target));
    }

    #[test]
    fn consistency_fails_when_pk_mismatches_c3a() {
        let link = C0ToC3aPkCommitmentLink;
        let pk = make_field(42);
        let source_values = vec![pk];
        let target = c3_signals(make_field(99), make_field(99));
        assert!(!link.check_consistency(&source_values, &target));
    }

    #[test]
    fn consistency_passes_when_pk_matches_c3b() {
        let link = C0ToC3bPkCommitmentLink;
        let pk = make_field(7);
        let source_values = vec![pk];
        let target = c3_signals(pk, make_field(55));
        assert!(link.check_consistency(&source_values, &target));
    }

    #[test]
    fn consistency_fails_when_pk_mismatches_c3b() {
        let link = C0ToC3bPkCommitmentLink;
        let pk = make_field(7);
        let source_values = vec![pk];
        let target = c3_signals(make_field(8), make_field(55));
        assert!(!link.check_consistency(&source_values, &target));
    }

    #[test]
    fn empty_source_is_inconsistent() {
        let link = C0ToC3aPkCommitmentLink;
        // Empty source values means malformed proof — should be inconsistent
        assert!(!link.check_consistency(&[], &[0u8; 64]));
    }

    #[test]
    fn short_target_signals_is_inconsistent() {
        let link = C0ToC3aPkCommitmentLink;
        // Target too short for 2 input fields
        assert!(!link.check_consistency(&[make_field(1)], &[0u8; 31]));
    }

    #[test]
    fn short_source_signals_returns_empty() {
        let link = C0ToC3aPkCommitmentLink;
        assert!(link.extract_source_values(&[0u8; 16]).is_empty());
    }
}
