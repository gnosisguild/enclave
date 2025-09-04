// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

/// This module defines event payloads that will dcrypt a ciphertext with a threshold quorum of decryption shares
use crate::{helpers::try_poly_from_bytes, ArcBytes, PartyId, TrBFVConfig};
use anyhow::*;
use fhe::bfv::Encoding;
use fhe::{bfv::Ciphertext, trbfv::ShareManager};
use fhe_traits::DeserializeParametrized;
use fhe_traits::FheDecoder;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// All decryption shares from a threshold quorum of nodes polys.
    pub d_share_polys: Vec<(PartyId, ArcBytes)>,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<ArcBytes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Response {
    /// The resultant plaintext
    pub plaintext: Vec<ArcBytes>,
}

pub async fn calculate_threshold_decryption(req: Request) -> Result<Response> {
    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let mut ordered_polys = req.d_share_polys;
    ordered_polys.sort_by_key(|&(key, _)| key);
    let d_share_polys = ordered_polys
        .into_iter()
        .map(|(_, bytes)| -> Result<_> {
            let poly = try_poly_from_bytes(&bytes, &params)?;
            Ok(poly)
        })
        .collect::<Result<Vec<_>>>()?;

    let ciphertexts = req
        .ciphertexts
        .into_iter()
        .map(|ciphertext| {
            Ciphertext::from_bytes(&ciphertext, &params).context("cannot deserialize ciphertext")
        })
        .collect::<Result<Vec<_>>>()?;

    let open_results = ciphertexts
        .into_iter()
        .map(|ciphertext| {
            let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());

            // TODO: should probably not need to clone here...
            share_manager
                .decrypt_from_shares(d_share_polys.clone(), Arc::new(ciphertext))
                .context("Could not decrypt ciphertext")
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Response {
        plaintext: open_results
            .into_iter()
            .map(|open_result| -> Result<_> {
                let plaintext = Vec::<u64>::try_decode(&open_result, Encoding::poly())
                    .context("could not decode plaintext")?;
                let bytes = bincode::serialize(&plaintext)?;
                Ok(Arc::new(bytes))
            })
            .collect::<Result<_>>()?,
    })
}
