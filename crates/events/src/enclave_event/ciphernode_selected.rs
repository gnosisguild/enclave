// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, Seed};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    sync::Arc,
};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeSelected {
    pub e3_id: E3id,
    pub threshold_m: usize,
    pub threshold_n: usize,
    pub seed: Seed,
    pub error_size: Arc<Vec<u8>>,
    pub esi_per_ct: usize,
    pub params: Arc<Vec<u8>>,
    pub party_id: u64,
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
