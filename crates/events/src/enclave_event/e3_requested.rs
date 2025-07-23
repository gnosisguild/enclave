// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, Seed};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct E3Requested {
    pub e3_id: E3id,
    pub threshold_m: usize,
    pub seed: Seed,
    pub params: Vec<u8>,
}

impl Display for E3Requested {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, threshold_m: {}, seed: {}, params: <omitted>",
            self.e3_id, self.threshold_m, self.seed
        )
    }
}
