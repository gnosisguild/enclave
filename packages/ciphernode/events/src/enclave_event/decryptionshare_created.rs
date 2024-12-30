use crate::E3id;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "anyhow::Result<()>")]
pub struct DecryptionshareCreated {
    pub decryption_share: Vec<u8>,
    pub e3_id: E3id,
    pub node: String,
}

impl Display for DecryptionshareCreated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}, node: {}", self.e3_id, self.node,)
    }
}
