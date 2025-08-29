// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{ArcBytes, SharedRng, TrBFVConfig};
use anyhow::{Context, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_rs::trbfv::{smudging::SmudgingNoiseGenerator, ShareManager};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// Error Size extracted from the E3 Program Parameters
    pub error_size: ArcBytes,
    /// Smudging noise per ciphertext
    pub esi_per_ct: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// The smudging noise shares
    pub esi_sss: Vec<SensitiveBytes>,
}

pub async fn gen_esi_sss(rng: &SharedRng, cipher: &Cipher, req: Request) -> Result<Response> {
    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let error_size = BigUint::from_bytes_be(&req.error_size);
    let esi_per_ct = req.esi_per_ct as usize;
    let esi_sss = (0..esi_per_ct)
        .map(|_| -> Result<_> {
            let generator = SmudgingNoiseGenerator::new(params.clone(), error_size.clone());
            let esi_coeffs = {
                generator
                    .generate_smudging_error(&mut *rng.lock().unwrap())
                    .context("Failed to generate smudging error")?
            };
            let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());
            let esi_poly = share_manager.bigints_to_poly(&esi_coeffs).unwrap();
            {
                share_manager
                    .generate_secret_shares_from_poly(esi_poly, &mut *rng.lock().unwrap())
                    .context("Failed to generate secret shares from poly")
            }
        })
        .collect::<Result<Vec<Vec<_>>>>()?;

    let esi_sss_result = esi_sss
        .into_iter()
        .map(|s| SensitiveBytes::new(bincode::serialize(&s)?, &cipher))
        .collect::<Result<_>>();

    Ok(Response {
        esi_sss: esi_sss_result?,
    })
}
