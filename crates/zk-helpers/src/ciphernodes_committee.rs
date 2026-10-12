// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_committee::{CiphernodesCommittee, CiphernodesCommitteeSize};

/// Validate `(T, N, H)` against the canonical committee table used by compiled Noir circuits.
///
/// Returns the canonical row on success. Callers must pass the same `(threshold_m, threshold_n,
/// h)` the active `committee/*/mod.nr` was built with — smudging bounds and C1 public IO depend on
/// `N_PARTIES` (= `n`), not BFV preset degree.
pub fn canonical_committee_for_circuit(
    committee: &CiphernodesCommittee,
) -> Result<CiphernodesCommittee, anyhow::Error> {
    let expected =
        CiphernodesCommitteeSize::from_threshold(committee.threshold, committee.n)?.values();
    if committee.h != expected.h {
        anyhow::bail!(
            "committee.h={} does not match canonical h={} for (T={}, N={})",
            committee.h,
            expected.h,
            committee.threshold,
            committee.n
        );
    }
    if committee.n != expected.n || committee.threshold != expected.threshold {
        anyhow::bail!(
            "committee (T={}, N={}, H={}) is not a canonical committee size",
            committee.threshold,
            committee.n,
            committee.h
        );
    }
    Ok(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_committee_for_circuit_accepts_micro_row() {
        let committee = CiphernodesCommitteeSize::Micro.values();
        assert_eq!(
            canonical_committee_for_circuit(&committee).unwrap(),
            committee
        );
    }
}
