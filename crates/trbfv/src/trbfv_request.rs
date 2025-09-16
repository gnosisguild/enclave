// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    calculate_decryption_key::{CalculateDecryptionKeyRequest, CalculateDecryptionKeyResponse},
    calculate_decryption_share::{
        CalculateDecryptionShareRequest, CalculateDecryptionShareResponse,
    },
    calculate_threshold_decryption::{
        CalculateThresholdDecryptionRequest, CalculateThresholdDecryptionResponse,
    },
    gen_esi_sss::{GenEsiSssRequest, GenEsiSssResponse},
    gen_pk_share_and_sk_sss::{GenPkShareAndSkSssRequest, GenPkShareAndSkSssResponse},
};

// NOTE: All size values use u64 instead of usize to maintain a stable
// protocol that works across different architectures. Convert these
// u64 values to usize when entering the library's internal APIs.

/// Input format for TrBFVRequest
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVRequest {
    GenEsiSss(GenEsiSssRequest),
    GenPkShareAndSkSss(GenPkShareAndSkSssRequest),
    CalculateDecryptionKey(CalculateDecryptionKeyRequest),
    CalculateDecryptionShare(CalculateDecryptionShareRequest),
    CalculateThresholdDecryption(CalculateThresholdDecryptionRequest),
}

/// Result format for TrBFVResponse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVResponse {
    GenEsiSss(GenEsiSssResponse),
    GenPkShareAndSkSss(GenPkShareAndSkSssResponse),
    CalculateDecryptionKey(CalculateDecryptionKeyResponse),
    CalculateDecryptionShare(CalculateDecryptionShareResponse),
    CalculateThresholdDecryption(CalculateThresholdDecryptionResponse),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVError {
    GenEsiSss,
    GenPkShareAndSkSss,
    CalculateDecryptionKey,
    CalculateDecryptionShare,
    CalculateThresholdDecryption,
}

impl std::error::Error for TrBFVError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            _ => None,
        }
    }
}

impl fmt::Display for TrBFVError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrBFVError::GenEsiSss => write!(f, "GenEsiSss"),
            TrBFVError::GenPkShareAndSkSss => write!(f, "GenPkShareAndSkSss"),
            TrBFVError::CalculateDecryptionKey => write!(f, "CalculateDecryptionKey"),
            TrBFVError::CalculateDecryptionShare => write!(f, "CalculateDecryptionShare"),
            TrBFVError::CalculateThresholdDecryption => write!(f, "CalculateThresholdDecryption"),
        }
    }
}
