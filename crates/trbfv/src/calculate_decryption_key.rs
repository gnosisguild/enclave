// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::TrBFVConfig;
use anyhow::Result;
/// This module defines event payloads that will generate the decryption key material to create a decryption share
use anyhow::*;
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_rs::trbfv::ShareManager;
use fhe_traits::Serialize;
use ndarray::Array2;
use zeroize::Zeroizing;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// All collected secret key shamir shares
    pub sk_sss_collected: Vec<SensitiveBytes>,
    /// All collected smudging noise shamir shares
    pub esi_sss_collected: Vec<Vec<SensitiveBytes>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Response {
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
    /// A single summed polynomial for this partys smudging noise
    pub es_poly_sum: Vec<SensitiveBytes>,
}

fn deserialize_to_array2(value: Zeroizing<Vec<u8>>) -> Result<Array2<u64>> {
    bincode::deserialize(&value).context("Error deserializing share")
}

pub async fn calculate_decryption_key(cipher: &Cipher, req: Request) -> Result<Response> {
    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());

    // convert to collected
    let sk_sss_collected = SensitiveBytes::access_vec(req.sk_sss_collected, cipher)?
        .into_iter()
        .map(deserialize_to_array2)
        .collect::<Result<Vec<_>>>()?;

    let es_sss_collected = req
        .esi_sss_collected
        .into_iter()
        .map(|sensitive_vec| -> Result<_> {
            SensitiveBytes::access_vec(sensitive_vec, cipher)?
                .into_iter()
                .map(deserialize_to_array2)
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;

    let sk_poly_sum = share_manager.aggregate_collected_shares(&sk_sss_collected)?;
    let es_poly_sum = es_sss_collected
        .into_iter()
        .map(|shares| -> Result<_> {
            let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());
            share_manager
                .aggregate_collected_shares(&shares)
                .context("Failed to aggregate es_sss")
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Response {
        es_poly_sum: SensitiveBytes::try_from_vec(
            es_poly_sum
                .into_iter()
                .map(|s| s.to_bytes())
                .collect::<Vec<_>>(),
            cipher,
        )?,
        sk_poly_sum: SensitiveBytes::new(sk_poly_sum.to_bytes(), &cipher)?,
    })
}
