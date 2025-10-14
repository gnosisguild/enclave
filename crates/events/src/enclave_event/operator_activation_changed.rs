use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct OperatorActivationChanged {
    pub operator: String,
    pub active: bool,
}
