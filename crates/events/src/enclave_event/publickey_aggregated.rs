// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, OrderedSet};
use actix::Message;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    #[derivative(Debug(format_with = "crate::hexf"))]
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
    pub nodes: OrderedSet<String>,
}

impl Display for PublicKeyAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, nodes: <omitted>, pubkey: <omitted>",
            self.e3_id,
        )
    }
}
