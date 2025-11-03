// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use derivative::Derivative;
use e3_utils::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
#[rtype(result = "()")]
pub struct PlaintextAggregated {
    pub e3_id: E3id,
    pub decrypted_output: ArcBytes,
}

impl Display for PlaintextAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
