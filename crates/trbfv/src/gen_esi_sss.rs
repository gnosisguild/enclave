// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    helpers::try_poly_from_bytes,
    shares::{Encrypted, SharedSecret},
    TrBFVConfig,
};
use anyhow::{Context, Result};
use e3_crypto::Cipher;
use e3_utils::{utility_types::ArcBytes, SharedRng};
use fhe::trbfv::ShareManager;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenEsiSssRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// This is pre-generated smudging noise.
    pub e_sm_raw: ArcBytes,
}

struct InnerRequest {
    pub trbfv_config: TrBFVConfig,
    pub e_sm_raw: ArcBytes,
}

impl TryFrom<GenEsiSssRequest> for InnerRequest {
    type Error = anyhow::Error;
    fn try_from(value: GenEsiSssRequest) -> std::result::Result<Self, Self::Error> {
        Ok(InnerRequest {
            trbfv_config: value.trbfv_config,
            e_sm_raw: value.e_sm_raw,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenEsiSssResponse {
    /// The smudging noise shares
    pub esi_sss: Vec<Encrypted<SharedSecret>>,
}

impl TryFrom<(InnerResponse, &Cipher)> for GenEsiSssResponse {
    type Error = anyhow::Error;
    fn try_from(
        (value, cipher): (InnerResponse, &Cipher),
    ) -> std::result::Result<Self, Self::Error> {
        Ok(GenEsiSssResponse {
            esi_sss: value
                .esi_sss
                .into_iter()
                .map(|s| Encrypted::new(s, cipher))
                .collect::<Result<_>>()?,
        })
    }
}

struct InnerResponse {
    pub esi_sss: Vec<SharedSecret>,
}

/// This function generates secret shares for the smudging noise (esi_sss) using the provided pre-generated smudging noise polynomial (e_sm_raw).
/// When implementing multiple ciphertext outputs decryptions, we are going to need multiple smudging noise polynomials,
/// so we are generating a vector of smudging noise secret shares (esi_sss) instead of just one in anticipation of that change.
/// We will also need to ensure that all of them are committed to the pk_generation circuit.
pub fn gen_esi_sss(
    rng: &SharedRng,
    cipher: &Cipher,
    req: GenEsiSssRequest,
) -> Result<GenEsiSssResponse> {
    info!("gen_esi_sss");
    let req: InnerRequest = req.try_into()?;

    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let e_sm_raw = req.e_sm_raw;

    info!("gen_esi_sss:mapping...");
    let e_sm_poly = try_poly_from_bytes(&e_sm_raw, &params)?;
    let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());

    info!("gen_esi_sss:generate_smudging_error...");

    let esi_sss = vec![SharedSecret::from(
        share_manager
            .generate_secret_shares_from_poly(e_sm_poly.into(), &mut *rng.lock().unwrap())
            .context("Failed to generate secret shares from poly")?,
    )];

    info!("gen_esi_sss:returning...");

    (InnerResponse { esi_sss }, cipher).try_into()
}
