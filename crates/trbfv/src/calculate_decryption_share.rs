// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use crate::helpers::{try_poly_from_sensitive_bytes, try_polys_from_sensitive_bytes_vec};
/// This module defines event payloads that will generate a decryption share for the given ciphertext for this node
use crate::TrBFVConfig;
use anyhow::*;
use e3_crypto::{Cipher, SensitiveBytes};
use e3_utils::utility_types::ArcBytes;
use fhe::{bfv::Ciphertext, trbfv::ShareManager};
use fhe_math::rq::Poly;
use fhe_traits::DeserializeParametrized;
use fhe_traits::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateDecryptionShareRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<ArcBytes>,
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
    /// A vector of summed polynomials for this parties smudging noise
    pub es_poly_sum: Vec<SensitiveBytes>,
}

struct InnerRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<Ciphertext>,
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: Poly,
    /// A vector of summed polynomials for this parties smudging noise
    pub es_poly_sum: Vec<Poly>,
}

impl TryFrom<(&Cipher, CalculateDecryptionShareRequest)> for InnerRequest {
    type Error = anyhow::Error;
    fn try_from(
        value: (&Cipher, CalculateDecryptionShareRequest),
    ) -> std::result::Result<InnerRequest, Self::Error> {
        let trbfv_config = value.1.trbfv_config.clone();
        let ciphertexts = value
            .1
            .ciphertexts
            .into_iter()
            .map(|ciphertext| {
                Ciphertext::from_bytes(&ciphertext, &trbfv_config.params())
                    .context("cannot deserialize ciphertext")
            })
            .collect::<Result<Vec<_>>>()?;

        let sk_poly_sum =
            try_poly_from_sensitive_bytes(value.1.sk_poly_sum, trbfv_config.params(), value.0)?;
        let es_poly_sum = try_polys_from_sensitive_bytes_vec(
            value.1.es_poly_sum,
            trbfv_config.params(),
            value.0,
        )?;

        Ok(InnerRequest {
            sk_poly_sum,
            es_poly_sum,
            ciphertexts,
            trbfv_config,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateDecryptionShareResponse {
    /// The decryption share for the given ciphertext
    pub d_share_poly: Vec<ArcBytes>,
}

struct InnerResponse {
    pub d_share_poly: Vec<Poly>,
}

impl From<InnerResponse> for CalculateDecryptionShareResponse {
    fn from(value: InnerResponse) -> Self {
        CalculateDecryptionShareResponse {
            d_share_poly: value
                .d_share_poly
                .into_iter()
                .map(|p| ArcBytes::from_bytes(p.to_bytes()))
                .collect(),
        }
    }
}

pub fn calculate_decryption_share(
    cipher: &Cipher,
    req: CalculateDecryptionShareRequest,
) -> Result<CalculateDecryptionShareResponse> {
    let req: InnerRequest = (cipher, req).try_into()?;

    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let threshold = req.trbfv_config.threshold() as usize;
    let params = req.trbfv_config.params();
    let sk_poly_sum = req.sk_poly_sum;
    let es_poly_sum = req.es_poly_sum;

    let d_share_poly = req
        .ciphertexts
        .into_iter()
        .enumerate()
        .map(|(idx, ciphertext)| {
            let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());
            share_manager
                .decryption_share(
                    Arc::new(ciphertext),
                    sk_poly_sum.clone(),
                    es_poly_sum[idx].clone(),
                )
                .context(format!("Could not decrypt ciphertext {}", idx))
        })
        .collect::<Result<Vec<Poly>>>()?;

    Ok(InnerResponse { d_share_poly }.into())
}
