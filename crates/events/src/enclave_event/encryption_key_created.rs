// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::E3id;
use actix::Message;
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    sync::Arc,
};

#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct EncryptionKey {
    pub party_id: u64,
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub pk_bfv: ArcBytes,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EncryptionKeyCreated {
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
    pub external: bool,
}

impl Display for EncryptionKeyCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
