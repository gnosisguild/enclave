// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C1 (PkGeneration) → C5 (PkAggregation) pk_commitment consistency link.
//!
//! ## Circuit layouts
//!
//! **C1 (PkGeneration)** outputs `(sk_commitment, pk_commitment, e_sm_commitment)`.
//! Public signals contain 3 fields (no public inputs); `pk_commitment` is at
//! field index 1 (byte offset 32..64).
//!
//! **C5 (PkAggregation)** takes `expected_threshold_pk_commitments: pub [Field; H]`
//! as public inputs and returns `pk_agg_commitment` as a single public output.
//! Public signals contain H+1 fields; the first H fields are per-party
//! `pk_commitment` values and the last field is the aggregated commitment.
//!
//! ## Check
//!
//! Each cipher node's C1 `pk_commitment` must appear somewhere in C5's
//! `expected_threshold_pk_commitments` array.

use super::{CommitmentLink, FieldValue, LinkScope};
use e3_events::ProofType;

/// Size of one BN254 field element in bytes.
const FIELD_SIZE: usize = 32;

/// C1 → C5 pk_commitment consistency link.
pub struct C1ToC5PkCommitmentLink;

impl CommitmentLink for C1ToC5PkCommitmentLink {
    fn name(&self) -> &'static str {
        "C1→C5 pk_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C1PkGeneration
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C5PkAggregation
    }

    fn scope(&self) -> LinkScope {
        LinkScope::CrossParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        // C1 outputs: (sk_commitment, pk_commitment, e_sm_commitment) — 3 fields, no public inputs
        // pk_commitment is at field index 1 (bytes 32..64)
        if public_signals.len() < 3 * FIELD_SIZE {
            return vec![];
        }
        let mut value = [0u8; FIELD_SIZE];
        value.copy_from_slice(&public_signals[FIELD_SIZE..2 * FIELD_SIZE]);
        vec![value]
    }

    fn check_consistency(
        &self,
        source_values: &[FieldValue],
        target_public_signals: &[u8],
    ) -> bool {
        if source_values.is_empty() || target_public_signals.len() < 2 * FIELD_SIZE {
            // Not enough data to perform the check — treat as vacuously consistent.
            return true;
        }

        let source_pk_commitment = &source_values[0];

        // C5 public_signals: [expected_pk_commitments[0..H], pk_agg_commitment]
        // H = total_fields - 1 (last field is the output)
        let total_fields = target_public_signals.len() / FIELD_SIZE;
        if total_fields < 2 {
            return true;
        }
        let h = total_fields - 1;

        // Check if the source pk_commitment appears in any of the H input fields
        for i in 0..h {
            let offset = i * FIELD_SIZE;
            if target_public_signals[offset..offset + FIELD_SIZE] == *source_pk_commitment {
                return true;
            }
        }

        false
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

    #[test]
    fn extract_pk_commitment_from_c1() {
        let link = C1ToC5PkCommitmentLink;
        let sk = make_field(1);
        let pk = make_field(2);
        let esm = make_field(3);
        let mut signals = Vec::new();
        signals.extend_from_slice(&sk);
        signals.extend_from_slice(&pk);
        signals.extend_from_slice(&esm);

        let values = link.extract_source_values(&signals);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], pk);
    }

    #[test]
    fn consistency_passes_when_pk_present_in_c5() {
        let link = C1ToC5PkCommitmentLink;
        let pk = make_field(42);
        let source_values = vec![pk];

        // C5: [pk_comm_0, pk_comm_1(=42), pk_agg_commitment]
        let mut c5_signals = Vec::new();
        c5_signals.extend_from_slice(&make_field(10));
        c5_signals.extend_from_slice(&pk);
        c5_signals.extend_from_slice(&make_field(99));

        assert!(link.check_consistency(&source_values, &c5_signals));
    }

    #[test]
    fn consistency_fails_when_pk_missing_from_c5() {
        let link = C1ToC5PkCommitmentLink;
        let pk = make_field(42);
        let source_values = vec![pk];

        // C5: [pk_comm_0, pk_comm_1, pk_agg_commitment] — neither matches 42
        let mut c5_signals = Vec::new();
        c5_signals.extend_from_slice(&make_field(10));
        c5_signals.extend_from_slice(&make_field(20));
        c5_signals.extend_from_slice(&make_field(99));

        assert!(!link.check_consistency(&source_values, &c5_signals));
    }

    #[test]
    fn short_signals_treated_as_consistent() {
        let link = C1ToC5PkCommitmentLink;
        // Too short for C1
        assert!(link.extract_source_values(&[0u8; 60]).is_empty());
        // Too short for C5
        assert!(link.check_consistency(&[make_field(1)], &[0u8; 31]));
    }
}
