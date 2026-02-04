// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// @todo this must be integrated inside Ciphernodes & Smart Contract
/// instead of being a separate type in here. The pvss crate should import this and
/// the default values that must be used and shared among the whole enclave repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiphernodesCommitteeSize {
    /// Small committee size (fast local/testing).
    Small,
    /// Medium committee size (default).
    Medium,
    /// Large committee size (higher assurance).
    Large,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiphernodesCommittee {
    /// Total number of parties (N_PARTIES).
    pub n: usize,
    /// Number of honest parties (H).
    pub h: usize,
    /// Threshold value (T).
    pub threshold: usize,
}

impl CiphernodesCommitteeSize {
    /// Returns `(num_parties, num_honest_parties, threshold)` for this size.
    pub fn values(self) -> CiphernodesCommittee {
        match self {
            CiphernodesCommitteeSize::Small => CiphernodesCommittee {
                n: 5,
                h: 3,
                threshold: 2,
            },
            _ => unreachable!(),
        }
        // @todo add the other committee sizes
        // CiphernodesCommitteeSize::Medium => CiphernodesCommittee {
        //     n: 5,
        //     h: 3,
        //     threshold: 2,
        // },
        // CiphernodesCommitteeSize::Large => CiphernodesCommittee {
        //     n: 5,
        //     h: 3,
        //     threshold: 2,
        // },
    }
}
