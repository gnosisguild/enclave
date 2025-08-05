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

/// Reference count bytes so event can be cloned and shared between threads
pub type Bytes = Arc<Vec<u8>>;

/// Input format for TrBFVRequest
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVRequest {
    GenerateEsiShares {
        params: Bytes,
        num_summed: u64,
    },
    GenerateSecretShares {
        params: Bytes,
    },
    Decrypt {
        params: Bytes,
        d_share_polys: Vec<Bytes>,
    },
    GenerateDecryptionKey {
        sk_sss_collected: Vec<SensitiveBytes>,
        esi_sss_collected: Vec<SensitiveBytes>,
    },
    GenerateDecryptionShare {
        params: Bytes,
        ciphertext: Bytes,
        sk_poly_sum: SensitiveBytes,
        es_poly_sum: SensitiveBytes,
    },
    ThresholdDecrypt {
        d_share_polys: Vec<SensitiveBytes>,
    },
}

/// Result format for TrBFVRequest
pub enum TrBFVResult {
    GenerateEsiShares {
        esi_sss: Vec<SensitiveBytes>,
    },
    GenerateSecretShares {
        pk_share: Bytes,
        sk_sss: Vec<SensitiveBytes>,
    },
    GenerateDecryptionKey {
        sk_poly_sum: SensitiveBytes,
        es_poly_sum: SensitiveBytes,
    },
    GenerateDecryptionShare {
        d_share_poly: Bytes,
    },
    ThresholdDecrypt {
        result: Bytes,
    },
}

/// The compute instruction for a threadpool computation
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeRequest {
    TrBFV(TrBFVRequest),
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ComputeRequested {
    correlation_id: CorrelationId,
    instruction: ComputeRequest,
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
