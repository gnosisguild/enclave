// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

pub type ArcBytes = Arc<Vec<u8>>;

/// Semantic PartyId
pub type PartyId = u64;

/// Convenience struct for holding threshold BFV configuration parameters
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrBFVConfig {
    /// BFV Params
    params: ArcBytes,
    /// Number of ciphernodes
    num_parties: u64,
    /// Threshold required
    threshold: u64,
}

impl TrBFVConfig {
    /// Constructor for the TrBFVConfig
    pub fn new(params: ArcBytes, num_parties: u64, threshold: u64) -> Self {
        Self {
            params,
            num_parties,
            threshold,
        }
    }

    pub fn params(&self) -> ArcBytes {
        self.params.clone() // NOTE: It might make sense to deserialize
                            // stright to BfvParameters here
                            // but leaving like this for now
    }

    pub fn num_parties(&self) -> u64 {
        self.num_parties
    }

    pub fn threshold(&self) -> u64 {
        self.threshold
    }
}

pub struct GenEsiSssResponse {
    pub esi_sss: Vec<Vec<u8>>,
}
pub async fn gen_esi_sss(
    trbfv_config: TrBFVConfig,
    error_size: ArcBytes,
    esi_per_ct: u64,
) -> GenEsiSssResponse {
    GenEsiSssResponse {
        esi_sss: vec![vec![]],
    }
}

pub struct GenPkShareAndSkSssResponse {
    pub pk_share: Vec<u8>,
    pub sk_sss: Vec<Vec<u8>>,
}
pub async fn gen_pk_share_and_sk_sss(
    trbfv_config: TrBFVConfig,
    crp: ArcBytes,
) -> GenPkShareAndSkSssResponse {
    GenPkShareAndSkSssResponse {
        pk_share: vec![],
        sk_sss: vec![vec![]],
    }
}

pub struct CalculateDecryptionKeyResponse {
    pub sk_poly_sum: Vec<u8>,
    pub es_poly_sum: Vec<Vec<u8>>,
}
pub async fn calculate_decryption_key(
    trbfv_config: TrBFVConfig,
    sk_sss_collected: Vec<Vec<u8>>,
    esi_sss_collected: Vec<Vec<u8>>,
) -> CalculateDecryptionKeyResponse {
    CalculateDecryptionKeyResponse {
        sk_poly_sum: vec![],
        es_poly_sum: vec![],
    }
}

pub struct CalculateDecryptionShareResponse {
    pub d_share_poly: Vec<Vec<u8>>,
}
pub async fn calculate_decryption_share(
    trbfv_config: TrBFVConfig,
    ciphertext: ArcBytes,
    sk_poly_sum: Vec<u8>,
    es_poly_sum: Vec<Vec<u8>>,
) -> CalculateDecryptionShareResponse {
    CalculateDecryptionShareResponse {
        d_share_poly: vec![vec![]],
    }
}

pub struct CalculateThresholdDecryptionResponse {
    pub plaintext: Vec<u8>,
}
pub async fn calculate_threshold_decryption(
    trbfv_config: TrBFVConfig,
    d_share_polys: Vec<(PartyId, ArcBytes)>,
) -> CalculateThresholdDecryptionResponse {
    CalculateThresholdDecryptionResponse { plaintext: vec![] }
}
