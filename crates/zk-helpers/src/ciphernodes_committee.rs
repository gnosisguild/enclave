// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// @todo this must be integrated inside Ciphernodes & Smart Contract
/// instead of being a separate type in here. The pvss crate should import this and
/// the default values that must be used and shared among the whole enclave repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CiphernodesCommitteeSize {
    /// Tiny committee size (for quick local testing with production parameters).
    Micro,
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
    /// Derives the committee size from threshold values (M, N).
    pub fn from_threshold(threshold_m: usize, threshold_n: usize) -> Result<Self> {
        match (threshold_m, threshold_n) {
            (1, 3) => Ok(Self::Micro),
            (2, 5) => Ok(Self::Small),
            (4, 10) => Ok(Self::Medium),
            (7, 20) => Ok(Self::Large),
            _ => bail!(
                "Unknown committee size for threshold ({}, {})",
                threshold_m,
                threshold_n
            ),
        }
    }

    /// Lower-case name as written into `circuits/bin/.active-preset.json` and the
    /// `--committee` flag of `scripts/build-circuits.ts`. Use this for stamp/env cross-checks.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Micro => "micro",
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    /// Returns `(num_parties, num_honest_parties, threshold)` for this size.
    pub fn values(self) -> CiphernodesCommittee {
        match self {
            CiphernodesCommitteeSize::Micro => CiphernodesCommittee {
                n: 3,
                h: 3,
                threshold: 1,
            },
            CiphernodesCommitteeSize::Small => CiphernodesCommittee {
                n: 5,
                h: 5,
                threshold: 2,
            },
            CiphernodesCommitteeSize::Medium => CiphernodesCommittee {
                n: 10,
                h: 8,
                threshold: 4,
            },
            CiphernodesCommitteeSize::Large => CiphernodesCommittee {
                n: 20,
                h: 15,
                threshold: 7,
            },
        }
    }
}

impl FromStr for CiphernodesCommitteeSize {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "micro" => Ok(Self::Micro),
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            _ => bail!("Unknown committee size '{s}'. Expected micro|small|medium|large"),
        }
    }
}

impl fmt::Display for CiphernodesCommitteeSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
