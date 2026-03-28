// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C0 (PkBfv) → C3 (ShareEncryption) pk_commitment consistency link.
//!
//! ## Circuit layouts
//!
//! **C0 (PkBfv)** outputs `(pk_commitment)`.
//! `pk_commitment` is extracted from the tail of `public_signals` via the
//! output layout.
//!
//! **C3 (ShareEncryption)** takes `(expected_pk_commitment, expected_message_commitment)`
//! as public inputs and produces no outputs (`CircuitOutputLayout::None`).
//! `expected_pk_commitment` is the first 32 bytes of `public_signals`.
//!
//! ## Scope
//!
//! Cross-party: C0 is from the verifying node (proving its own BFV public key),
//! while C3a/C3b are from other committee members (proving they encrypted
//! shares using that key).
//!
//! ## Check
//!
//! The `pk_commitment` output from C0 must equal the `expected_pk_commitment`
//! public input in C3.

use super::{CommitmentLink, FieldValue, LinkScope};
use e3_events::{CircuitName, ProofType};
use e3_zk_helpers::FIELD_BYTE_LEN;

/// C0 → C3a pk_commitment consistency link.
pub struct C0ToC3aPkCommitmentLink;

/// C0 → C3b pk_commitment consistency link.
pub struct C0ToC3bPkCommitmentLink;

fn extract_c0_pk_commitment(public_signals: &[u8]) -> Vec<FieldValue> {
    let layout = CircuitName::PkBfv.output_layout();
    let Some(bytes) = layout.extract_field(public_signals, "pk_commitment") else {
        return vec![];
    };
    let mut value = [0u8; FIELD_BYTE_LEN];
    value.copy_from_slice(bytes);
    vec![value]
}

/// C3 has no outputs (`CircuitOutputLayout::None`), so all public_signals are
/// inputs. The layout is `[expected_pk_commitment, expected_message_commitment]`,
/// making `expected_pk_commitment` the first 32 bytes.
fn check_c3_pk_commitment(source_values: &[FieldValue], target_public_signals: &[u8]) -> bool {
    if source_values.is_empty() {
        return true;
    }
    if target_public_signals.len() < FIELD_BYTE_LEN {
        return false;
    }
    source_values[0] == target_public_signals[..FIELD_BYTE_LEN]
}

impl CommitmentLink for C0ToC3aPkCommitmentLink {
    fn name(&self) -> &'static str {
        "C0→C3a pk_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C0PkBfv
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C3aSkShareEncryption
    }

    fn scope(&self) -> LinkScope {
        LinkScope::CrossParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        extract_c0_pk_commitment(public_signals)
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        check_c3_pk_commitment(source_values, target_public_signals)
    }
}

impl CommitmentLink for C0ToC3bPkCommitmentLink {
    fn name(&self) -> &'static str {
        "C0→C3b pk_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C0PkBfv
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C3bESmShareEncryption
    }

    fn scope(&self) -> LinkScope {
        LinkScope::CrossParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        extract_c0_pk_commitment(public_signals)
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        check_c3_pk_commitment(source_values, target_public_signals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_field(val: u8) -> [u8; 32] {
        let mut f = [0u8; 32];
        f[31] = val;
        f
    }

    /// Build C0 public_signals: [pub_input(s)...] [pk_commitment output].
    /// C0 has 1 output field; prepend some pub input bytes to simulate real signals.
    fn make_c0_signals(pk_commitment: [u8; 32]) -> Vec<u8> {
        // Simulate 1 pub input field + 1 output field = 64 bytes
        let mut signals = vec![0xAAu8; 32];
        signals.extend_from_slice(&pk_commitment);
        signals
    }

    /// Build C3 public_signals: [expected_pk_commitment, expected_message_commitment].
    /// C3 has no outputs, only 2 input fields = 64 bytes.
    fn make_c3_signals(expected_pk_commitment: [u8; 32]) -> Vec<u8> {
        let mut signals = Vec::new();
        signals.extend_from_slice(&expected_pk_commitment);
        signals.extend_from_slice(&make_field(0xFF)); // expected_message_commitment
        signals
    }

    #[test]
    fn extract_pk_commitment_from_c0() {
        let link = C0ToC3aPkCommitmentLink;
        let pk = make_field(42);
        let signals = make_c0_signals(pk);
        let values = link.extract_source_values(&signals);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], pk);
    }

    #[test]
    fn consistency_passes_when_pk_matches() {
        let pk = make_field(42);
        let source_values = vec![pk];
        let c3_signals = make_c3_signals(pk);

        assert!(C0ToC3aPkCommitmentLink.check_consistency(&source_values, &c3_signals));
        assert!(C0ToC3bPkCommitmentLink.check_consistency(&source_values, &c3_signals));
    }

    #[test]
    fn consistency_fails_when_pk_differs() {
        let source_values = vec![make_field(42)];
        let c3_signals = make_c3_signals(make_field(99));

        assert!(!C0ToC3aPkCommitmentLink.check_consistency(&source_values, &c3_signals));
        assert!(!C0ToC3bPkCommitmentLink.check_consistency(&source_values, &c3_signals));
    }

    #[test]
    fn empty_source_is_vacuously_consistent() {
        let c3_signals = make_c3_signals(make_field(1));
        assert!(C0ToC3aPkCommitmentLink.check_consistency(&[], &c3_signals));
    }

    #[test]
    fn short_c0_signals_extract_empty() {
        let link = C0ToC3aPkCommitmentLink;
        // Too short for C0 output extraction
        assert!(link.extract_source_values(&[0u8; 16]).is_empty());
    }

    #[test]
    fn short_c3_signals_treated_as_inconsistent() {
        let source_values = vec![make_field(1)];
        // Only 16 bytes — too short for expected_pk_commitment
        assert!(!C0ToC3aPkCommitmentLink.check_consistency(&source_values, &[0u8; 16]));
    }
}
