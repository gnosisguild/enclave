// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! C1 (PkGeneration) → C2a (ShareComputation) secret commitment link.
//!
//! ## Circuit layouts
//!
//! **C1 (PkGeneration)** outputs `(sk_commitment, pk_commitment)`.
//! Public signals contain 2 fields (no public inputs):
//! - field 0: `sk_commitment`   (byte offset   0..32)
//! - field 1: `pk_commitment`   (byte offset  32..64)
//!
//! **C2a** takes `expected_secret_commitment` first, then returns N outgoing key
//! commitments and N*L packed-message commitments.
//!
//! ## Checks
//!
//! - **C1→C2a**: `C1.sk_commitment` must equal `C2a.expected_secret_commitment`.
//!   Prevents a party from Shamir-splitting a different sk than the one committed
//!   to in their C1 (TrBFV pk_generation) proof.
//!
//! Synchronized C1/C6 do not expose an ESM anchor. Old proof-type discriminants
//! remain reserved in the event schema; their links are not part of this protocol.

use super::{CommitmentLink, FieldValue, LinkScope};
use e3_events::{CircuitName, ProofType};
use e3_zk_config::FIELD_BYTE_LEN;

/// C1 → C2a: `sk_commitment` from PkGeneration must match C2a's `expected_secret_commitment`.
pub struct C1ToC2aSkCommitmentLink;

impl CommitmentLink for C1ToC2aSkCommitmentLink {
    fn name(&self) -> &'static str {
        "C1->C2a sk_commitment"
    }

    fn source_proof_type(&self) -> ProofType {
        ProofType::C1PkGeneration
    }

    fn target_proof_type(&self) -> ProofType {
        ProofType::C2aSkShareComputation
    }

    fn scope(&self) -> LinkScope {
        LinkScope::SameParty
    }

    fn extract_source_values(&self, public_signals: &[u8]) -> Vec<FieldValue> {
        let layout = CircuitName::PkGeneration.output_layout();
        let Some(bytes) = layout.extract_field(public_signals, "sk_commitment") else {
            return vec![];
        };
        let mut value = [0u8; FIELD_BYTE_LEN];
        value.copy_from_slice(bytes);
        vec![value]
    }

    fn check_signals(&self, source_values: &[FieldValue], target_public_signals: &[u8]) -> bool {
        if source_values.is_empty() || target_public_signals.len() < FIELD_BYTE_LEN {
            return false;
        }
        target_public_signals[..FIELD_BYTE_LEN] == source_values[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c1_c2_commitment_widths_agree_for_every_supported_pair() {
        use e3_committee::CiphernodesCommitteeSize;
        use e3_fhe_params::BfvPreset;
        use e3_zk_config::{dkg, threshold};

        for preset in BfvPreset::PAIR_PRESETS {
            let (params, _) = preset.build_pair().unwrap();
            for committee in [
                CiphernodesCommitteeSize::Minimum,
                CiphernodesCommitteeSize::Micro,
                CiphernodesCommitteeSize::Small,
            ] {
                let c1 =
                    threshold::pk_generation::Configs::new(preset, &params, committee).unwrap();
                let c2 = dkg::share_computation::Configs::new(preset, &params, committee).unwrap();
                assert_eq!(
                    c1.bits.sk_bit, c2.bits.bit_sk_secret,
                    "{preset:?}/{committee:?}"
                );
                assert_eq!(
                    c1.bits.e_sm_bit, c2.bits.bit_e_sm_secret,
                    "{preset:?}/{committee:?}"
                );
            }
        }
    }

    fn make_field(val: u8) -> [u8; 32] {
        let mut f = [0u8; 32];
        f[31] = val;
        f
    }

    /// C1 public signals: [sk_commitment, pk_commitment].
    fn c1_signals(sk: [u8; 32], pk: [u8; 32]) -> Vec<u8> {
        let mut v = Vec::with_capacity(64);
        v.extend_from_slice(&sk);
        v.extend_from_slice(&pk);
        v
    }

    /// C2 inner public signals: [expected_secret_commitment] + share commitments...
    fn c2_signals(secret_commitment: [u8; 32], share_commitments: &[[u8; 32]]) -> Vec<u8> {
        let mut v = Vec::with_capacity(32 + share_commitments.len() * 32);
        v.extend_from_slice(&secret_commitment);
        for c in share_commitments {
            v.extend_from_slice(c);
        }
        v
    }

    // ── C1→C2a ──────────────────────────────────────────────────────────────

    #[test]
    fn extract_sk_commitment_from_c1() {
        let link = C1ToC2aSkCommitmentLink;
        let sk = make_field(1);
        let values = link.extract_source_values(&c1_signals(sk, make_field(2)));
        assert_eq!(values.len(), 1);
        assert_eq!(values[0], sk);
    }

    #[test]
    fn c2a_consistency_passes_when_sk_matches() {
        let link = C1ToC2aSkCommitmentLink;
        let sk = make_field(42);
        let c2 = c2_signals(sk, &[make_field(10), make_field(11)]);
        assert!(link.check_signals(&[sk], &c2));
    }

    #[test]
    fn c2a_consistency_fails_when_sk_differs() {
        let link = C1ToC2aSkCommitmentLink;
        let c2 = c2_signals(make_field(99), &[make_field(10)]);
        assert!(!link.check_signals(&[make_field(42)], &c2));
    }

    #[test]
    fn c2a_short_or_empty_signals() {
        let link = C1ToC2aSkCommitmentLink;
        // C1 too short to extract sk_commitment
        assert!(link.extract_source_values(&[0u8; 60]).is_empty());
        // Empty source values
        assert!(!link.check_signals(&[], &c2_signals(make_field(1), &[])));
        // C2 target too short
        assert!(!link.check_signals(&[make_field(1)], &[0u8; 10]));
    }
}
