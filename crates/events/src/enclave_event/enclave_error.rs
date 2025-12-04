// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Message;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

use crate::{BusHandle, ErrorDispatcher};

pub trait FromError {
    fn from_error(err_type: EType, error: impl Into<String>) -> Self;
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EnclaveError {
    pub err_type: EType,
    pub message: String,
}

impl Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EType {
    Evm,
    KeyGeneration,
    PublickeyAggregation,
    IO,
    PlaintextAggregation,
    Decryption,
    Sortition,
    Data,
    Event,
}

impl EnclaveError {
    pub fn new(err_type: EType, message: impl Into<anyhow::Error>) -> Self {
        Self {
            err_type,
            message: message.into().to_string(),
        }
    }
}

impl FromError for EnclaveError {
    fn from_error(err_type: EType, error: impl Into<String>) -> Self {
        Self {
            err_type,
            message: error.into(),
        }
    }
}

/// Function to run a closure that returns a result. If result is an Err variant it is trapped and
/// sent to the bus as an ErrorEvent
pub fn trap<F>(err_type: EType, bus: &BusHandle, runner: F)
where
    F: FnOnce() -> anyhow::Result<()>,
{
    match runner() {
        Ok(_) => (),
        Err(e) => bus.err(err_type, e),
    }
}
