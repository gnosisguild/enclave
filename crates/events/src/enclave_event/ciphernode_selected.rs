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
pub struct CiphernodeSelected {
    pub e3_id: E3id,
    pub threshold_m: usize,
    pub threshold_n: usize,
    pub seed: Seed,
    #[derivative(Debug(format_with = "crate::hexf"))]
    pub error_size: Arc<Vec<u8>>,
    pub esi_per_ct: usize,
    #[derivative(Debug(format_with = "crate::hexf"))]
    pub params: Arc<Vec<u8>>,
    pub party_id: u64,
}

impl Default for CiphernodeSelected {
    fn default() -> Self {
        CiphernodeSelected {
            e3_id: E3id::new("99", 0),
            error_size: Arc::new(vec![]),
            esi_per_ct: 0,
            params: Arc::new(vec![]),
            party_id: 0,
            seed: Seed([0u8; 32]),
            threshold_m: 0,
            threshold_n: 0,
        }
    }
}

impl Display for CiphernodeSelected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, threshold_m: {}, threshold_n: {}",
            self.e3_id, self.threshold_m, self.threshold_n
        )
    }
}
