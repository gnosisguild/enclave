// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C6 (ShareDecryption) → C7 (DecryptedSharesAggregation) `d_commitment` consistency link.
//!
//! ## Circuit layouts
//!
//! **C6 (ThresholdShareDecryption)** outputs `d_commitment`.
//!
//! **C7 (DecryptedSharesAggregation)** `main` has (in order) `expected_d_commitments`,
//! `party_ids`, and `message` as public inputs; there are no public return values. So
//! `public_signals` are:
//! `[d_commitments (T+1)] [party_ids (T+1)] [message coefficients (MAX_MSG_NON_ZERO_COEFFS)] [c0]`.
//!
//! We recover `T+1` as `(total_fields - MAX_MSG_NON_ZERO_COEFFS - 1) / 2`; see **Caveat** below and
//! `circuits/bin/threshold/decrypted_shares_aggregation/src/main.nr`.
//!
//! ## Check
//!
//! The C6 `d_commitment` must appear in the `expected_d_commitments` prefix only (not in party IDs
//! or message).
//!
//! ## Caveat
//!
//! The Rust constant `MAX_MSG_NON_ZERO_COEFFS` (imported from
//! `e3_zk_config`) must match the **compiled**
//! C7 Noir circuit (same value as `Polynomial<MAX_MSG_NON_ZERO_COEFFS>` /
//! `lib::configs::default::MAX_MSG_NON_ZERO_COEFFS` for that artifact). If you ever ship multiple C7
//! builds with different message widths, verifying or linking against the wrong artifact will make
//! `(total_fields - MAX_MSG_NON_ZERO_COEFFS - 1) / 2` wrong and this check will mis-parse
//! `public_signals` (false negatives/positives relative to the intended circuit).

use super::{CommitmentLink, FieldValue, LinkScope};
use e3_events::{CircuitName, ProofType};
use e3_zk_config::FIELD_BYTE_LEN;
use e3_zk_config::MAX_MSG_NON_ZERO_COEFFS;

/// C6 → C7 `d_commitment` consistency link.
pub struct C6ToC7DCommitmentLink;

impl CommitmentLink for C6ToC7DCommitmentLink {
    fn name(&self) -> &'static str {
        "C6->C7 d_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C6ThresholdShareDecryption
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C7DecryptedSharesAggregation
    }

    fn scope(&self) -> LinkScope {
        LinkScope::CrossParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        let layout = CircuitName::ThresholdShareDecryption.output_layout();
        let Some(bytes) = layout.extract_field(public_signals, "d_commitment") else {
            return vec![];
        };
        let mut value = [0u8; FIELD_BYTE_LEN];
        value.copy_from_slice(bytes);
        vec![value]
    }

    fn applies_to(&self, source: &[u8], target: &[u8]) -> bool {
        let source = super::c2_to_c3::deduplicate(source);
        let target = super::c2_to_c3::deduplicate(target);
        if !target.len().is_multiple_of(FIELD_BYTE_LEN) {
            return false;
        }
        let Some(count) = (target.len() / FIELD_BYTE_LEN).checked_sub(MAX_MSG_NON_ZERO_COEFFS + 1)
        else {
            return false;
        };
        if count == 0 || !count.is_multiple_of(2) {
            return false;
        }
        let s = count / 2;
        let n = 2 * s - 1;
        source.len() == (7 + s + 2 * n) * FIELD_BYTE_LEN
            && source[6 * FIELD_BYTE_LEN..(6 + s) * FIELD_BYTE_LEN]
                == target[s * FIELD_BYTE_LEN..2 * s * FIELD_BYTE_LEN]
            && source[2 * FIELD_BYTE_LEN..3 * FIELD_BYTE_LEN]
                == target[target.len() - FIELD_BYTE_LEN..]
    }

    fn check_signals(&self, source_values: &[FieldValue], target_public_signals: &[u8]) -> bool {
        if source_values.is_empty() {
            return false;
        }

        if !target_public_signals.len().is_multiple_of(FIELD_BYTE_LEN) {
            return false;
        }

        // C7: `circuits/.../decrypted_shares_aggregation/src/main.nr` — public inputs in order:
        // (T+1) d commitments, (T+1) party IDs, `MAX_MSG_NON_ZERO_COEFFS` message coefficients.
        let total_fields = target_public_signals.len() / FIELD_BYTE_LEN;
        let rem = total_fields.checked_sub(MAX_MSG_NON_ZERO_COEFFS + 1);
        let Some(rem) = rem else {
            return false;
        };
        if rem % 2 != 0 {
            return false;
        }
        let d_commitment_fields = rem / 2;
        if d_commitment_fields == 0 {
            return false;
        }

        let source_d_commitment = &source_values[0];

        for i in 0..d_commitment_fields {
            let offset = i * FIELD_BYTE_LEN;
            if target_public_signals[offset..offset + FIELD_BYTE_LEN] == *source_d_commitment {
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

    /// Builds C7 `public_signals` bytes: `(T+1)` d commitments, `(T+1)` party IDs, then message
    /// coeffs (`MAX_MSG_NON_ZERO_COEFFS` fields), matching `decrypted_shares_aggregation/src/main.nr`.
    fn c7_public_signals(
        d_commitments: &[[u8; FIELD_BYTE_LEN]],
        party_ids: &[[u8; FIELD_BYTE_LEN]],
    ) -> Vec<u8> {
        assert_eq!(d_commitments.len(), party_ids.len());
        let mut v = Vec::new();
        for c in d_commitments {
            v.extend_from_slice(c);
        }
        for p in party_ids {
            v.extend_from_slice(p);
        }
        for _ in 0..MAX_MSG_NON_ZERO_COEFFS {
            v.extend_from_slice(&make_field(0));
        }
        v.extend_from_slice(&make_field(0)); // c0-prefix commitment
        v
    }

    #[test]
    fn extract_d_commitment_from_c6() {
        let link = C6ToC7DCommitmentLink;
        let d = make_field(7);
        let signals = d.to_vec();

        let values = link.extract_source_values(&signals);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], d);
    }

    #[test]
    fn consistency_passes_when_d_present_in_c7_inputs() {
        let link = C6ToC7DCommitmentLink;
        let d = make_field(42);
        let source_values = vec![d];

        let d_comm = [make_field(10), d, make_field(99)];
        let party = [make_field(1), make_field(2), make_field(3)];
        let c7_signals = c7_public_signals(&d_comm, &party);

        assert!(link.check_signals(&source_values, &c7_signals));
    }

    #[test]
    fn consistency_fails_when_d_missing_from_commitments() {
        let link = C6ToC7DCommitmentLink;
        let d = make_field(42);
        let source_values = vec![d];

        let d_comm = [make_field(10), make_field(20), make_field(99)];
        let party = [make_field(1), make_field(2), make_field(3)];
        let c7_signals = c7_public_signals(&d_comm, &party);

        assert!(!link.check_signals(&source_values, &c7_signals));
    }

    #[test]
    fn consistency_fails_when_d_only_appears_in_message_tail() {
        let link = C6ToC7DCommitmentLink;
        let d = make_field(42);
        let source_values = vec![d];

        let d_comm = [make_field(10), make_field(20), make_field(99)];
        let party = [make_field(1), make_field(2), make_field(3)];
        let mut c7_signals = c7_public_signals(&d_comm, &party);
        // Overwrite first message coefficient with `d` — must not count as a match.
        let msg_off = 6 * FIELD_BYTE_LEN;
        c7_signals[msg_off..msg_off + FIELD_BYTE_LEN].copy_from_slice(&d);

        assert!(!link.check_signals(&source_values, &c7_signals));
    }

    #[test]
    fn short_source_signals_treated_as_inconsistent() {
        let link = C6ToC7DCommitmentLink;
        assert!(link.extract_source_values(&[0u8; 16]).is_empty());
        assert!(!link.check_signals(&[], &[0u8; 32]));
    }

    #[test]
    fn short_target_signals_treated_as_inconsistent() {
        let link = C6ToC7DCommitmentLink;
        assert!(!link.check_signals(&[make_field(1)], &[0u8; 16]));
    }

    #[test]
    fn replaced_sets_and_other_ciphertexts_are_not_comparable() {
        let link = C6ToC7DCommitmentLink;
        let ids = [make_field(1), make_field(3)];
        let mut target = c7_public_signals(&[make_field(10), make_field(20)], &ids);
        let mut source = vec![0; 15 * FIELD_BYTE_LEN];
        source[6 * FIELD_BYTE_LEN..8 * FIELD_BYTE_LEN].copy_from_slice(&ids.concat());
        assert!(link.applies_to(&source, &target));
        target[3 * FIELD_BYTE_LEN + 31] = 2;
        assert!(
            !link.applies_to(&source, &target),
            "a later set cannot accuse an earlier share"
        );
        target[3 * FIELD_BYTE_LEN + 31] = 3;
        let last = target.len() - 1;
        target[last] = 1;
        assert!(
            !link.applies_to(&source, &target),
            "another ciphertext is not a negative lookup"
        );
    }
}
