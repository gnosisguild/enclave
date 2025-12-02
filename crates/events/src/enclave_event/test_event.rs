// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct TestEvent {
    pub msg: String,
    pub entropy: u64,
}

impl TestEvent {
    pub fn new(msg: &str, entropy: u64) -> Self {
        Self {
            msg: msg.to_owned(),
            entropy,
        }
    }
}

#[cfg(test)]
use std::fmt::{self, Display};

#[cfg(test)]
impl Display for TestEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestEvent(msg: {})", self.msg)
    }
}
