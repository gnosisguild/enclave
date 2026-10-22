// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Public IO layout for [`CircuitName::NodeFold`] (must stay aligned with `node_fold/src/main.nr`).

use crate::circuits::utils::bytes_to_field_strings;
use crate::error::ZkError;
use e3_events::{CircuitName, DkgFoldAggCommits, Proof};

/// Total public field count for `node_fold` at committee size `n`, honest `h`, threshold moduli `l`.
pub fn node_fold_public_field_count(n: usize, h: usize, l: usize) -> usize {
    10 + 2 * n + h + (n + h) * l
}

fn field_hex_to_bytes32(field: &str) -> Result<[u8; 32], ZkError> {
    let s = field.strip_prefix("0x").unwrap_or(field);
    if s.len() > 64 {
        return Err(ZkError::InvalidInput(format!(
            "field hex too long for bytes32: {field}"
        )));
    }
    let mut out = [0u8; 32];
    let decoded = hex::decode(s).map_err(|e| ZkError::InvalidInput(e.to_string()))?;
    let start = 32usize.saturating_sub(decoded.len());
    out[start..].copy_from_slice(&decoded);
    Ok(out)
}

fn field_hex_to_u64(field: &str) -> Result<u64, ZkError> {
    let s = field.strip_prefix("0x").unwrap_or(field);
    let trimmed = s.trim_start_matches('0');
    let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
    u64::from_str_radix(trimmed, 16).map_err(|e| ZkError::InvalidInput(e.to_string()))
}

/// Read `party_id`, `sk_agg_commit`, and `esm_agg_commit` from a `NodeFold` proof.
pub fn extract_node_fold_agg_commits(
    proof: &Proof,
    committee_n: usize,
    committee_h: usize,
    n_moduli: usize,
) -> Result<(u64, DkgFoldAggCommits), ZkError> {
    if proof.circuit != CircuitName::NodeFold {
        return Err(ZkError::InvalidInput(format!(
            "expected NodeFold proof, got {}",
            proof.circuit
        )));
    }
    let fields = bytes_to_field_strings(proof.public_signals.as_ref())?;
    let expected = node_fold_public_field_count(committee_n, committee_h, n_moduli);
    if fields.len() != expected {
        return Err(ZkError::InvalidInput(format!(
            "NodeFold public field count {} != expected {} (n={committee_n}, h={committee_h}, l={n_moduli})",
            fields.len(),
            expected
        )));
    }
    let party_id = field_hex_to_u64(&fields[0])?;
    let sk_index = 9 + committee_n + (committee_n + committee_h) * n_moduli;
    let sk_agg_commit = field_hex_to_bytes32(&fields[sk_index])?;
    let row = fields[sk_index + 1..sk_index + 1 + committee_n]
        .iter()
        .map(|field| field_hex_to_bytes32(field))
        .collect::<Result<Vec<_>, _>>()?;
    let prf_key_commits_hash = alloy::primitives::keccak256(row.concat()).into();
    Ok((
        party_id,
        DkgFoldAggCommits {
            sk_agg_commit,
            prf_key_commits_hash,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_utils::ArcBytes;

    #[test]
    fn extracts_expected_fields_from_golden_layout_vector() {
        // Golden layout vector: verifies positional extraction
        // (party_id at index 0, sk/esm commits at tail).
        let n = 3usize;
        let h = 3usize;
        let l = 2usize;
        let field_count = node_fold_public_field_count(n, h, l);

        let mut fields = vec![[0u8; 32]; field_count];
        fields[0][31] = 2; // party_id = 2
        let sk_index = 9 + n + (n + h) * l;
        fields[sk_index] = [0x11; 32];
        fields[sk_index + 1..sk_index + 1 + n].fill([0x22; 32]);

        let mut public_signals = Vec::with_capacity(field_count * 32);
        for f in fields {
            public_signals.extend_from_slice(&f);
        }

        let proof = Proof::new(
            CircuitName::NodeFold,
            ArcBytes::from_bytes(&[]),
            ArcBytes::from_bytes(&public_signals),
        );

        let (party_id, commits) =
            extract_node_fold_agg_commits(&proof, n, h, l).expect("extract should succeed");
        assert_eq!(party_id, 2);
        assert_eq!(commits.sk_agg_commit, [0x11; 32]);
        assert_eq!(
            commits.prf_key_commits_hash,
            <[u8; 32]>::from(alloy::primitives::keccak256(vec![0x22; 32 * n]))
        );
    }
}
