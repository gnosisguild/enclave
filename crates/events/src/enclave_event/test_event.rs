use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct TestEvent {
    pub msg: String,
    pub entropy: u64,
}

#[cfg(test)]
use std::fmt::{self, Display};

#[cfg(test)]
impl Display for TestEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TestEvent(msg: {})", self.msg)
    }
}
