// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{ArcBytes, SharedRng, TrBFVConfig};
use anyhow::{Context, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_rs::trbfv::{smudging::SmudgingNoiseGenerator, ShareManager};
use ndarray::Array2;
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

struct InnerRequest {
    pub trbfv_config: TrBFVConfig,
    pub error_size: BigUint,
    pub esi_per_ct: u64,
}

impl TryFrom<Request> for InnerRequest {
    type Error = anyhow::Error;
    fn try_from(value: Request) -> std::result::Result<Self, Self::Error> {
        Ok(InnerRequest {
            trbfv_config: value.trbfv_config,
            error_size: BigUint::from_bytes_be(&value.error_size),
            esi_per_ct: value.esi_per_ct,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// The smudging noise shares
    pub esi_sss: Vec<Vec<SensitiveBytes>>,
}

impl TryFrom<(InnerResponse, &Cipher)> for Response {
    type Error = anyhow::Error;
    fn try_from(
        (value, cipher): (InnerResponse, &Cipher),
    ) -> std::result::Result<Self, Self::Error> {
        Ok(Response {
            esi_sss: value
                .esi_sss
                .into_iter()
                .map(|s| SensitiveBytes::try_from_unserialized_vec(s, cipher))
                .collect::<Result<_>>()?,
        })
    }
}

struct InnerResponse {
    pub esi_sss: Vec<Vec<Array2<u64>>>,
}

pub async fn gen_esi_sss(rng: &SharedRng, cipher: &Cipher, req: Request) -> Result<Response> {
    let req = InnerRequest::try_from(req)?;

    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let error_size = req.error_size;
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

    Ok(Response::try_from((InnerResponse { esi_sss }, cipher))?)
}
