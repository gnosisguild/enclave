// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod pk_bfv;
pub mod registry;
pub mod sample;
pub mod traits;

/// Variant for input types for DKG.
///
/// This variant is used to determine the type of input that is used for the DKG
/// circuits (C2, C3, C4)
#[derive(Clone)]
pub enum DkgInputType {
    /// The input type that generates shares of a secret key using secret sharing.
    SecretKey,
    /// The input type that generates shares of smudging noise instead of secret key shares.
    SmudgingNoise,
}

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
    n: usize,
    /// Number of honest parties (H).
    h: usize,
    /// Threshold value (T).
    threshold: usize,
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
