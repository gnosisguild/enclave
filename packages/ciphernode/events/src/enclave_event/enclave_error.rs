use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

pub trait FromError {
    type Error;
    fn from_error(err_type: EnclaveErrorType, error: Self::Error) -> Self;
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EnclaveError {
    pub err_type: EnclaveErrorType,
    pub message: String,
}

impl Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnclaveErrorType {
    Evm,
    KeyGeneration,
    PublickeyAggregation,
    IO,
    PlaintextAggregation,
    Decryption,
    Sortition,
    Data,
}

impl EnclaveError {
    pub fn new(err_type: EnclaveErrorType, message: &str) -> Self {
        Self {
            err_type,
            message: message.to_string(),
        }
    }
}

impl FromError for EnclaveError {
    type Error = anyhow::Error;
    fn from_error(err_type: EnclaveErrorType, error: Self::Error) -> Self {
        Self {
            err_type,
            message: error.to_string(),
        }
    }
}
