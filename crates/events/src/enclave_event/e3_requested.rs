// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, Seed};
use actix::Message;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    sync::Arc,
};

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
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
    #[derivative(Debug(format_with = "crate::hexf"))]
    /// The error size for the FHE computation. This can be calculated for the E3 program based on
    /// the size of the ciphertext and the depth of the program [tbd add link]
    pub error_size: Arc<Vec<u8>>,
    /// The number of smudging noise per ciphertext.
    pub esi_per_ct: usize,
    /// The FHE parameters
    #[derivative(Debug(format_with = "crate::hexf"))]
    pub params: Arc<Vec<u8>>,
}

impl Default for E3Requested {
    fn default() -> Self {
        E3Requested {
            e3_id: E3id::new("99", 0),
            error_size: Arc::new(vec![]),
            esi_per_ct: 0,
            params: Arc::new(vec![]),
            seed: Seed([0u8; 32]),
            threshold_m: 0,
            threshold_n: 0,
        }
    }
}

impl Display for E3Requested {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, threshold_m: {}, threshold_n: {}, seed: {}, params: <omitted>",
            self.e3_id, self.threshold_m, self.threshold_n, self.seed
        )
    }
}
