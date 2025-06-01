use crate::E3id;
use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// E3RequestComplete event is a local only event
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct E3RequestComplete {
    pub e3_id: E3id,
}

impl Display for E3RequestComplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e3_id: {}", self.e3_id)
    }
}
