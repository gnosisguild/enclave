// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fmt, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::E3id;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateId(usize);

impl AggregateId {
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    pub fn to_usize(&self) -> usize {
        self.0
    }
}

impl From<Option<E3id>> for AggregateId {
    fn from(value: Option<E3id>) -> Self {
        if let Some(e3_id) = value {
            Self::new(e3_id.chain_id() as usize)
        } else {
            Self::new(0)
        }
    }
}

impl Deref for AggregateId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for AggregateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}
