// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};

use crate::{
    calculate_decryption_key, calculate_decryption_share, calculate_threshold_decryption,
    gen_esi_sss, gen_pk_share_and_sk_sss,
};

// NOTE: All size values use u64 instead of usize to maintain a stable
// protocol that works across different architectures. Convert these
// u64 values to usize when entering the library's internal APIs.

/// Input format for TrBFVRequest
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVRequest {
    GenEsiSss(gen_esi_sss::Request),
    GenPkShareAndSkSss(gen_pk_share_and_sk_sss::Request),
    CalculateDecryptionKey(calculate_decryption_key::Request),
    CalculateDecryptionShare(calculate_decryption_share::Request),
    CalculateThresholdDecryption(calculate_threshold_decryption::Request),
}

/// Result format for TrBFVResponse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVResponse {
    GenEsiSss(gen_esi_sss::Response),
    GenPkShareAndSkSss(gen_pk_share_and_sk_sss::Response),
    CalculateDecryptionKey(calculate_decryption_key::Response),
    CalculateDecryptionShare(calculate_decryption_share::Response),
    CalculateThresholdDecryption(calculate_threshold_decryption::Response),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVError {
    // TODO: Add errors here as required
}
