// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

/// This module defines event payloads that will generate a decryption share for the given ciphertext for this node
use crate::{ArcBytes, TrBFVConfig};
use anyhow::*;
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_math::rq::Poly;
use fhe_rs::bfv::BfvParameters;
use fhe_rs::{bfv::Ciphertext, trbfv::ShareManager};
use fhe_traits::DeserializeParametrized;
use fhe_traits::DeserializeWithContext;
use fhe_traits::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<ArcBytes>,
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
    /// A vector of summed polynomials for this parties smudging noise
    pub es_poly_sum: Vec<SensitiveBytes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Response {
    /// The decryption share for the given ciphertext
    pub d_share_poly: Vec<ArcBytes>,
}

fn try_poly_from_sensitive_bytes(
    bytes: SensitiveBytes,
    params: Arc<BfvParameters>,
    cipher: &Cipher,
) -> Result<Poly> {
    Ok(Poly::from_bytes(
        &bytes.access(cipher)?,
        &Arc::new(fhe_math::rq::Context::new(
            params.moduli(),
            params.degree(),
        )?),
    )?)
}

fn try_polys_from_sensitive_bytes_vec(
    bytes_vec: Vec<SensitiveBytes>,
    params: Arc<BfvParameters>,
    cipher: &Cipher,
) -> Result<Vec<Poly>> {
    bytes_vec
        .into_iter()
        .map(|s| try_poly_from_sensitive_bytes(s, params.clone(), cipher))
        .collect::<Result<Vec<_>>>()
}

pub async fn calculate_decryption_share(cipher: &Cipher, req: Request) -> Result<Response> {
    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let ciphertexts = req
        .ciphertexts
        .into_iter()
        .map(|ciphertext| {
            Ciphertext::from_bytes(&ciphertext, &params).context("cannot deserialize ciphertext")
        })
        .collect::<Result<Vec<_>>>()?;

    let sk_poly_sum = try_poly_from_sensitive_bytes(req.sk_poly_sum, params.clone(), cipher)?;
    let es_poly_sum = try_polys_from_sensitive_bytes_vec(req.es_poly_sum, params.clone(), cipher)?;
    let d_share_poly = ciphertexts
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

    Ok(Response {
        d_share_poly: d_share_poly
            .into_iter()
            .map(|p| Arc::new(p.to_bytes()))
            .collect(),
    })
}
