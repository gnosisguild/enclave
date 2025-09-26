// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Display;

#[derive(Derivative, Message, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub struct KeyshareCreated {
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
    pub node: String,
}

impl Display for KeyshareCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
