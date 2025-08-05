// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use crate::CorrelationId;
use actix::Message;
use e3_crypto::SensitiveBytes;
use serde::{Deserialize, Serialize};

pub type Bytes = Arc<Vec<u8>>;

///
pub enum TrBFVRequest {
    GenerateEsiShares {
        num_summed: u64,
    },
    GenerateSecretShares,
    Decrypt {
        d_share_polys: Vec<Bytes>,
    },
    GenerateDecryptionShare {
        ciphertext: Bytes,
        sk_poly_sum: SensitiveBytes,
        es_poly_sum: SensitiveBytes,
    },
}

pub enum TrBFVResult {
    GenerateEsiShares {},
}

/// The compute instruction for a threadpool computation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequest {}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequested {
    correlation_id: CorrelationId,
    instruction: ComputeRequest,
    input: SensitiveBytes,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestFailed {
    correlation_id: CorrelationId,
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequestSucceeded {
    correlation_id: CorrelationId,
}
