use crate::{E3id, OrderedSet};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PublicKeyAggregated {
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
    pub nodes: OrderedSet<String>,
    pub src_chain_id: u64,
}

impl Display for PublicKeyAggregated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, src_chain_id: {}, nodes: <omitted>, pubkey: <omitted>",
            self.e3_id, self.src_chain_id,
        )
    }
}
