use std::sync::Arc;

use fhe_rs::bfv::BfvParameters;
use serde::{Deserialize, Serialize};

use crate::ArcBytes;
use e3_bfv_helpers::decode_bfv_params_arc;

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

    pub fn params(&self) -> Arc<BfvParameters> {
        decode_bfv_params_arc(&self.params)
    }

    pub fn num_parties(&self) -> u64 {
        self.num_parties
    }

    pub fn threshold(&self) -> u64 {
        self.threshold
    }
}
