use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct CiphernodeAdded {
    pub address: String,
    pub index: usize,
    pub num_nodes: usize,
}

impl Display for CiphernodeAdded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "address: {}, index: {}, num_nodes: {}",
            self.address, self.index, self.num_nodes
        )
    }
}
