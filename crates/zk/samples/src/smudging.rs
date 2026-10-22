// SPDX-License-Identifier: LGPL-3.0-only

//! Additive smudging setup for artificial samples; each draw remains fresh.

use e3_fhe_params::{generate_smudging_error, BfvPreset};
use e3_zk_witness::WitnessError;
use fhe::bfv::BfvParameters;
use num_bigint::BigInt;
use rand::{CryptoRng, RngCore};
use std::sync::Arc;
/// Draws fresh smudging coefficients for one threshold-BFV operation.
pub(crate) struct SmudgingDrawer {
    params: Arc<BfvParameters>,
    parties: usize,
    ciphertexts: usize,
    preset: BfvPreset,
}

impl SmudgingDrawer {
    pub(crate) fn new(
        params: &Arc<BfvParameters>,
        parties: usize,
        ciphertexts: usize,
        preset: BfvPreset,
    ) -> Self {
        Self {
            params: params.clone(),
            parties,
            ciphertexts,
            preset,
        }
    }

    pub(crate) fn draw(
        &self,
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Vec<BigInt>, WitnessError> {
        let lambda = self.preset.metadata().lambda;
        generate_smudging_error(
            self.params.clone(),
            self.parties,
            self.ciphertexts,
            0,
            lambda,
            rng,
        )
        .map_err(|e| WitnessError::Sample(format!("Failed to generate smudging error: {:?}", e)))
    }
}
