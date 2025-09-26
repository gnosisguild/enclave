// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, Seed};
use actix::Message;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct E3Requested {
    /// The E3 round ID
    pub e3_id: E3id,
    /// The minimum number of shares required to decrypt a ciphertext
    pub threshold_m: usize,
    /// The total committee size for the round
    pub threshold_n: usize,
    /// A seed to provide randomness for the round
    pub seed: Seed,
    /// The error size for the FHE computation. This can be calculated for the E3 program based on
    /// the size of the ciphertext and the depth of the program [tbd add link]
    pub error_size: ArcBytes,
    /// The number of smudging noise per ciphertext.
    pub esi_per_ct: usize,
    /// The FHE parameters
    pub params: ArcBytes,
}

impl Default for E3Requested {
    fn default() -> Self {
        E3Requested {
            e3_id: E3id::new("99", 0),
            error_size: ArcBytes::from_bytes(vec![]),
            esi_per_ct: 0,
            params: ArcBytes::from_bytes(vec![]),
            seed: Seed([0u8; 32]),
            threshold_m: 0,
            threshold_n: 0,
        }
    }
}

impl Display for E3Requested {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
