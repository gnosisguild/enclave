// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub struct DecryptionshareCreated {
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub decryption_share: Vec<u8>,
    pub e3_id: E3id,
    pub node: String,
}

impl Display for DecryptionshareCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}, node: {}", self.e3_id, self.node,)
    }
}
