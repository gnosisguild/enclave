use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Represents a shutdown event triggered by SIG TERM
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct Shutdown;
impl Display for Shutdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shutdown",)
    }
}
