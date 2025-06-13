use alloy::primitives::U256;
use alloy_primitives::ruint::ParseError;
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct E3id {
    id: String,
    chain_id: u64,
}

impl fmt::Display for E3id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.chain_id, self.id)
    }
}

impl E3id {
    pub fn new(id: impl Into<String>, chain_id: u64) -> Self {
        Self {
            id: id.into(),
            chain_id,
        }
    }

    pub fn e3_id(&self) -> &str {
        &self.id
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

impl TryFrom<E3id> for U256 {
    type Error = ParseError;
    fn try_from(value: E3id) -> Result<Self, Self::Error> {
        U256::from_str_radix(&value.id, 10)
    }
}
