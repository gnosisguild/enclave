// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
//
//! Byte-accurate parsing of circuit public signals for cross-circuit chaining.
//!
//! Layouts are derived from Noir `main.nr` entry points. Each Field is 32 bytes (big-endian).
//!
//! - C0 (Pk)
//!   - Public output: 1 field = pk_commitment.
//!
//! - C1 (PkGeneration)
//!   - Public output: 3 fields at end = (sk_commitment, pk_commitment, e_sm_commitment).  
//!   - Index from end: 0 = last = e_sm_commitment, 1 = pk_commitment, 2 = sk_commitment.
//!
//! - C2 (ShareComputation) — intentionally skipped for now
//!   - Noir: `circuits/bin/dkg/share_computation/src/main.nr`
//!   - The **verifiable C2 proof** in the current protocol is the final wrapper proof, whose
//!     public outputs are only small wrapper metadata (e.g. `key_hash`, `final_commitment`).
//!   - The values we would need for direct C2→C3/C4 wiring (per-party/per-modulus commitments)
//!     are computed inside the C2 base circuits but are **not exposed** in the final proof’s
//!     `public_signals`.
//!   - Therefore, there is currently no C2 public-signal layout we can parse to enforce
//!     `C3.expected_message_commitment == C2.output[...]` without changing the Noir circuits.
//!     This is addressed later in the plan (Phase B: expose C2 outputs or add a compact root).
//!
//! - C3 (ShareEncryption)
//!   - Field 0 = expected_pk_commitment, Field 1 = expected_message_commitment.  
//!   - Rest = ct0is, ct1is (not needed for chaining).
//!
//! - C4 (DkgShareDecryption)
//!   - `expected_commitments: pub [[Field; L_THRESHOLD]; H]`, then `-> pub Field`  
//!   - Layout: row-major expected_commitments (H × L_THRESHOLD fields), then 1 return field.  
//!   - Total = H * L_THRESHOLD + 1 fields. Requires runtime H and L_THRESHOLD.
//!
//! - C5 (PkAggregation)
//!   - `expected_threshold_pk_commitments: pub [Field; H]`, then `-> pub Field`  
//!   - Layout: H fields (commitments), then 1 return (aggregated). Total = H + 1 fields.
//!
//! - C6 (ThresholdShareDecryption)
//!   - Field 0 = expected_sk_commitment, Field 1 = expected_e_sm_commitment.
//!
//! - C7 (DecryptedSharesAggregation)
//!   - Public inputs only (no public return):
//!     - `decryption_shares: pub [[Polynomial<MAX_MSG_NON_ZERO_COEFFS>; L]; T + 1]`
//!     - `party_ids: pub [Field; T + 1]`
//!     - `message: pub Polynomial<MAX_MSG_NON_ZERO_COEFFS>`
//!   - Layout is the concatenation of all public fields, in this order:
//!     1) decryption_shares (row-major party → modulus → coefficient)
//!     2) party_ids
//!     3) message coefficients
//!   - Total public fields:
//!     \[
//!       (T+1) * L * MAX_MSG_NON_ZERO_COEFFS \\, + \\, (T+1) \\, + \\, MAX_MSG_NON_ZERO_COEFFS
//!     \]

use crate::error::ZkError;

/// Size in bytes of a single Field in Noir public inputs/outputs.
pub const FIELD_SIZE: usize = 32;

fn check_length(signals: &[u8], min_fields: usize, label: &str) -> Result<(), ZkError> {
    let min_len = min_fields * FIELD_SIZE;
    if signals.len() < min_len {
        return Err(ZkError::InvalidInput(format!(
            "{}: expected at least {} bytes ({} fields), got {}",
            label,
            min_len,
            min_fields,
            signals.len()
        )));
    }
    if signals.len() % FIELD_SIZE != 0 {
        return Err(ZkError::InvalidInput(format!(
            "{}: length must be multiple of {}, got {}",
            label,
            FIELD_SIZE,
            signals.len()
        )));
    }
    Ok(())
}

/// Extract one field at `index` (0-based). Caller must ensure bounds.
fn field_at(signals: &[u8], index: usize) -> [u8; FIELD_SIZE] {
    let start = index * FIELD_SIZE;
    let mut out = [0u8; FIELD_SIZE];
    out.copy_from_slice(&signals[start..start + FIELD_SIZE]);
    out
}

/// Extract field at index from end (0 = last, 1 = second-to-last).
fn field_from_end(signals: &[u8], from_end: usize) -> [u8; FIELD_SIZE] {
    let total_fields = signals.len() / FIELD_SIZE;
    let index = total_fields - 1 - from_end;
    field_at(signals, index)
}

/// C0 public output: single pk_commitment (32 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C0PublicSignals {
    pub pk_commitment: [u8; FIELD_SIZE],
}

/// Parse C0 public signals. Exactly 1 field (32 bytes).
pub fn parse_c0(signals: &[u8]) -> Result<C0PublicSignals, ZkError> {
    check_length(signals, 1, "C0")?;
    Ok(C0PublicSignals {
        pk_commitment: field_at(signals, 0),
    })
}

/// C1 public output: three commitments at end of public signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C1PublicSignals {
    pub sk_commitment: [u8; FIELD_SIZE],
    pub pk_commitment: [u8; FIELD_SIZE],
    pub e_sm_commitment: [u8; FIELD_SIZE],
}

/// Parse C1 public signals. Expects at least 3 fields; reads last 3 as (sk, pk, e_sm) commitment.
pub fn parse_c1(signals: &[u8]) -> Result<C1PublicSignals, ZkError> {
    check_length(signals, 3, "C1")?;
    Ok(C1PublicSignals {
        sk_commitment: field_from_end(signals, 2),
        pk_commitment: field_from_end(signals, 1),
        e_sm_commitment: field_from_end(signals, 0),
    })
}

/// C3 public inputs used for chaining: first two fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C3PublicSignals {
    pub expected_pk_commitment: [u8; FIELD_SIZE],
    pub expected_message_commitment: [u8; FIELD_SIZE],
}

/// Parse C3 public signals (first 2 fields only). Rest is ct0is/ct1is.
pub fn parse_c3(signals: &[u8]) -> Result<C3PublicSignals, ZkError> {
    check_length(signals, 2, "C3")?;
    Ok(C3PublicSignals {
        expected_pk_commitment: field_at(signals, 0),
        expected_message_commitment: field_at(signals, 1),
    })
}

/// C4 public signals: 2D expected_commitments (row-major) plus return commitment.
#[derive(Debug, Clone)]
pub struct C4PublicSignals {
    /// Row-major: [party_idx][mod_idx]. Length = h * l_threshold.
    pub expected_commitments: Vec<[u8; FIELD_SIZE]>,
    /// Return value (last field).
    pub return_commitment: [u8; FIELD_SIZE],
}

/// Parse C4 public signals. Dimensions must match the circuit (H, L_THRESHOLD).
pub fn parse_c4(signals: &[u8], h: usize, l_threshold: usize) -> Result<C4PublicSignals, ZkError> {
    let expected_count = h * l_threshold;
    let total_fields = expected_count + 1;
    check_length(signals, total_fields, "C4")?;
    let mut expected_commitments = Vec::with_capacity(expected_count);
    for i in 0..expected_count {
        expected_commitments.push(field_at(signals, i));
    }
    let return_commitment = field_at(signals, expected_count);
    Ok(C4PublicSignals {
        expected_commitments,
        return_commitment,
    })
}

/// C5 public signals: H expected pk commitments plus aggregated return.
#[derive(Debug, Clone)]
pub struct C5PublicSignals {
    pub expected_threshold_pk_commitments: Vec<[u8; FIELD_SIZE]>,
    pub aggregated_pk_commitment: [u8; FIELD_SIZE],
}

/// Parse C5 public signals. `h` = number of parties (committee size for this circuit).
pub fn parse_c5(signals: &[u8], h: usize) -> Result<C5PublicSignals, ZkError> {
    let total_fields = h + 1;
    check_length(signals, total_fields, "C5")?;
    let mut expected_threshold_pk_commitments = Vec::with_capacity(h);
    for i in 0..h {
        expected_threshold_pk_commitments.push(field_at(signals, i));
    }
    let aggregated_pk_commitment = field_at(signals, h);
    Ok(C5PublicSignals {
        expected_threshold_pk_commitments,
        aggregated_pk_commitment,
    })
}

/// C6 public inputs used for chaining: first two fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C6PublicSignals {
    pub expected_sk_commitment: [u8; FIELD_SIZE],
    pub expected_e_sm_commitment: [u8; FIELD_SIZE],
}

/// Parse C6 public signals (first 2 fields only).
pub fn parse_c6(signals: &[u8]) -> Result<C6PublicSignals, ZkError> {
    check_length(signals, 2, "C6")?;
    Ok(C6PublicSignals {
        expected_sk_commitment: field_at(signals, 0),
        expected_e_sm_commitment: field_at(signals, 1),
    })
}

/// C7 public signals split into their three public components.
#[derive(Debug, Clone)]
pub struct C7PublicSignals {
    /// Flattened fields for `decryption_shares`, in Noir order:
    /// party-major → modulus-major → coefficient-major.
    ///
    /// Length = (t_plus_1 * l * max_msg_non_zero_coeffs).
    pub decryption_shares: Vec<[u8; FIELD_SIZE]>,
    /// Length = (t_plus_1).
    pub party_ids: Vec<[u8; FIELD_SIZE]>,
    /// Message polynomial coefficients. Length = (max_msg_non_zero_coeffs).
    pub message_coefficients: Vec<[u8; FIELD_SIZE]>,
}

/// Parse C7 public signals by slicing with the circuit dimensions.
///
/// - `t_plus_1`: equals `T + 1` in Noir (number of reconstructing parties).
/// - `l`: equals `L` in Noir (number of CRT moduli).
/// - `max_msg_non_zero_coeffs`: equals `MAX_MSG_NON_ZERO_COEFFS` in Noir (message poly length).
pub fn parse_c7(
    signals: &[u8],
    t_plus_1: usize,
    l: usize,
    max_msg_non_zero_coeffs: usize,
) -> Result<C7PublicSignals, ZkError> {
    let shares_fields = t_plus_1
        .checked_mul(l)
        .and_then(|x| x.checked_mul(max_msg_non_zero_coeffs))
        .ok_or_else(|| {
            ZkError::InvalidInput("C7: overflow computing shares field count".to_string())
        })?;
    let party_id_fields = t_plus_1;
    let msg_fields = max_msg_non_zero_coeffs;
    let total_fields = shares_fields + party_id_fields + msg_fields;

    check_length(signals, total_fields, "C7")?;

    let mut decryption_shares = Vec::with_capacity(shares_fields);
    for i in 0..shares_fields {
        decryption_shares.push(field_at(signals, i));
    }
    let mut party_ids = Vec::with_capacity(party_id_fields);
    for i in 0..party_id_fields {
        party_ids.push(field_at(signals, shares_fields + i));
    }
    let mut message_coefficients = Vec::with_capacity(msg_fields);
    for i in 0..msg_fields {
        message_coefficients.push(field_at(signals, shares_fields + party_id_fields + i));
    }

    Ok(C7PublicSignals {
        decryption_shares,
        party_ids,
        message_coefficients,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_size_is_32() {
        assert_eq!(FIELD_SIZE, 32);
    }

    #[test]
    fn c0_exactly_one_field() {
        let ok = [0u8; 32];
        let parsed = parse_c0(&ok).unwrap();
        assert_eq!(parsed.pk_commitment, ok);

        let short = [0u8; 16];
        assert!(parse_c0(&short).is_err());
        let long = [0u8; 64];
        let p = parse_c0(&long).unwrap();
        assert_eq!(p.pk_commitment, &long[0..32]);
    }

    #[test]
    fn c1_last_three_fields() {
        // 3 fields = 96 bytes
        let mut signals = [0u8; 96];
        signals[0..32].fill(1); // sk
        signals[32..64].fill(2); // pk
        signals[64..96].fill(3); // e_sm
        let p = parse_c1(&signals).unwrap();
        assert_eq!(p.sk_commitment, [1u8; 32]);
        assert_eq!(p.pk_commitment, [2u8; 32]);
        assert_eq!(p.e_sm_commitment, [3u8; 32]);

        assert!(parse_c1(&[0u8; 32]).is_err());
    }

    #[test]
    fn c3_first_two_fields() {
        let mut signals = [0u8; 64];
        signals[0..32].fill(10);
        signals[32..64].fill(20);
        let p = parse_c3(&signals).unwrap();
        assert_eq!(p.expected_pk_commitment, [10u8; 32]);
        assert_eq!(p.expected_message_commitment, [20u8; 32]);
        assert!(parse_c3(&[0u8; 32]).is_err());
    }

    #[test]
    fn c4_h2_l1() {
        // H=2, L_THRESHOLD=1 -> 2*1 + 1 = 3 fields = 96 bytes
        let mut signals = [0u8; 96];
        signals[0..32].fill(1);
        signals[32..64].fill(2);
        signals[64..96].fill(99); // return
        let p = parse_c4(&signals, 2, 1).unwrap();
        assert_eq!(p.expected_commitments.len(), 2);
        assert_eq!(p.expected_commitments[0], [1u8; 32]);
        assert_eq!(p.expected_commitments[1], [2u8; 32]);
        assert_eq!(p.return_commitment, [99u8; 32]);

        assert!(parse_c4(&signals, 2, 1).is_ok());
        assert!(parse_c4(&[0u8; 64], 2, 1).is_err());
    }

    #[test]
    fn c5_h3() {
        // H=3 -> 3 + 1 = 4 fields = 128 bytes
        let mut signals = [0u8; 128];
        signals[0..32].fill(1);
        signals[32..64].fill(2);
        signals[64..96].fill(3);
        signals[96..128].fill(42); // aggregated
        let p = parse_c5(&signals, 3).unwrap();
        assert_eq!(p.expected_threshold_pk_commitments.len(), 3);
        assert_eq!(p.expected_threshold_pk_commitments[0], [1u8; 32]);
        assert_eq!(p.expected_threshold_pk_commitments[1], [2u8; 32]);
        assert_eq!(p.expected_threshold_pk_commitments[2], [3u8; 32]);
        assert_eq!(p.aggregated_pk_commitment, [42u8; 32]);
        assert!(parse_c5(&[0u8; 32], 3).is_err());
    }

    #[test]
    fn c6_first_two_fields() {
        let mut signals = [0u8; 64];
        signals[0..32].fill(7);
        signals[32..64].fill(8);
        let p = parse_c6(&signals).unwrap();
        assert_eq!(p.expected_sk_commitment, [7u8; 32]);
        assert_eq!(p.expected_e_sm_commitment, [8u8; 32]);
        assert!(parse_c6(&[0u8; 32]).is_err());
    }

    #[test]
    fn c7_slices_three_segments() {
        // Tiny synthetic dimensions:
        // t_plus_1=2, l=3, max_msg_non_zero_coeffs=4
        // shares_fields = 2*3*4 = 24
        // party_ids = 2
        // message = 4
        // total = 30 fields = 960 bytes
        let t_plus_1 = 2;
        let l = 3;
        let max = 4;
        let total_fields = (t_plus_1 * l * max) + t_plus_1 + max;
        let mut signals = vec![0u8; total_fields * 32];

        // Fill shares fields with 1..=24 marker bytes per field (by first byte).
        for i in 0..(t_plus_1 * l * max) {
            signals[i * 32] = (i as u8) + 1;
        }
        // Fill party ids with 100, 101 markers.
        let party_start = (t_plus_1 * l * max) * 32;
        signals[party_start] = 100;
        signals[party_start + 32] = 101;
        // Fill message coeffs with 200..203 markers.
        let msg_start = party_start + (t_plus_1 * 32);
        for i in 0..max {
            signals[msg_start + i * 32] = 200 + (i as u8);
        }

        let p = parse_c7(&signals, t_plus_1, l, max).unwrap();
        assert_eq!(p.decryption_shares.len(), 24);
        assert_eq!(p.party_ids.len(), 2);
        assert_eq!(p.message_coefficients.len(), 4);
        assert_eq!(p.decryption_shares[0][0], 1);
        assert_eq!(p.decryption_shares[23][0], 24);
        assert_eq!(p.party_ids[0][0], 100);
        assert_eq!(p.party_ids[1][0], 101);
        assert_eq!(p.message_coefficients[0][0], 200);
        assert_eq!(p.message_coefficients[3][0], 203);

        // Too short
        assert!(parse_c7(&signals[..signals.len() - 32], t_plus_1, l, max).is_err());
    }

    #[test]
    fn invalid_length_not_multiple_of_32() {
        assert!(parse_c0(&[0u8; 33]).is_err());
        assert!(parse_c3(&[0u8; 40]).is_err());
    }
}
