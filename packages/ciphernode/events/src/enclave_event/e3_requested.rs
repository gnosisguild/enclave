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
    pub src_chain_id: u64,
}

impl Display for E3Requested {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "e3_id: {}, threshold_m: {}, src_chain_id: {}, seed: {}, params: <omitted>",
            self.e3_id, self.threshold_m, self.src_chain_id, self.seed
        )
    }
}
