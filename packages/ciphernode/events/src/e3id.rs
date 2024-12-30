use alloy::primitives::U256;
use alloy_primitives::ruint::ParseError;
use core::fmt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct E3id(pub String);
impl fmt::Display for E3id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl E3id {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl From<u32> for E3id {
    fn from(value: u32) -> Self {
        E3id::new(value.to_string())
    }
}

impl From<String> for E3id {
    fn from(value: String) -> Self {
        E3id::new(value)
    }
}

impl From<&str> for E3id {
    fn from(value: &str) -> Self {
        E3id::new(value)
    }
}

impl TryFrom<E3id> for U256 {
    type Error = ParseError;
    fn try_from(value: E3id) -> Result<Self, Self::Error> {
        U256::from_str_radix(&value.0, 10)
    }
}
