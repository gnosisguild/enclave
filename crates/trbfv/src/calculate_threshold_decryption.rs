// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

/// This module defines event payloads that will dcrypt a ciphertext with a threshold quorum of decryption shares
use crate::{helpers::try_poly_from_bytes, PartyId, TrBFVConfig};
use anyhow::*;
use e3_utils::utility_types::ArcBytes;
use fhe::bfv::{Encoding, Plaintext};
use fhe::{bfv::Ciphertext, trbfv::ShareManager};
use fhe_math::rq::Poly;
use fhe_traits::DeserializeParametrized;
use fhe_traits::FheDecoder;
use tracing::info;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateThresholdDecryptionRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// All decryption shares from a threshold quorum of nodes polys.
    pub d_share_polys: Vec<(PartyId, ArcBytes)>,
    /// One or more Ciphertexts to decrypt
    pub ciphertexts: Vec<ArcBytes>,
}

struct InnerRequest {
    trbfv_config: TrBFVConfig,
    d_share_polys: Vec<Poly>,
    ciphertexts: Vec<Ciphertext>,
}

impl TryFrom<CalculateThresholdDecryptionRequest> for InnerRequest {
    type Error = anyhow::Error;
    fn try_from(
        value: CalculateThresholdDecryptionRequest,
    ) -> std::result::Result<Self, Self::Error> {
        let trbfv_config = value.trbfv_config.clone();

        let params = value.trbfv_config.params();
        let ciphertexts = value
            .ciphertexts
            .into_iter()
            .map(|ciphertext| {
                Ciphertext::from_bytes(&ciphertext, &trbfv_config.params())
                    .context("cannot deserialize ciphertext")
            })
            .collect::<Result<Vec<_>>>()?;

        // Ensure the polys are ordered by party_id
        let mut ordered_polys = value.d_share_polys;
        ordered_polys.sort_by_key(|&(key, _)| key);
        let d_share_polys = ordered_polys
            .into_iter()
            .map(|(_, bytes)| -> Result<_> {
                let poly = try_poly_from_bytes(&bytes, &params)?;
                Ok(poly)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(InnerRequest {
            d_share_polys,
            ciphertexts,
            trbfv_config,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateThresholdDecryptionResponse {
    /// The resultant plaintext
    pub plaintext: Vec<ArcBytes>,
}

struct InnerResponse {
    plaintext: Vec<Plaintext>,
}

impl TryFrom<InnerResponse> for CalculateThresholdDecryptionResponse {
    type Error = anyhow::Error;
    fn try_from(value: InnerResponse) -> std::result::Result<Self, Self::Error> {
        Ok(CalculateThresholdDecryptionResponse {
            plaintext: value
                .plaintext
                .into_iter()
                .map(|open_result| -> Result<_> {
                    let plaintext = Vec::<u64>::try_decode(&open_result, Encoding::poly())
                        .context("could not decode plaintext")?;
                    let bytes = bincode::serialize(&plaintext)?;
                    Ok(ArcBytes::from_bytes(bytes))
                })
                .collect::<Result<_>>()?,
        })
    }
}
pub fn calculate_threshold_decryption(
    req: CalculateThresholdDecryptionRequest,
) -> Result<CalculateThresholdDecryptionResponse> {
    info!("Calculating threshold decryption...");
    let req: InnerRequest = req.try_into()?;

    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let d_share_polys = req.d_share_polys.clone();

    let plaintext = req
        .ciphertexts
        .into_iter()
        .map(|ciphertext| {
            let mut share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone());

            // TODO: should probably not need to clone here...
            info!("d_share_polys: {:?}", d_share_polys);
            info!("ciphertext: {:?}", ciphertext);

            share_manager
                .decrypt_from_shares(d_share_polys.clone(), Arc::new(ciphertext))
                .context("Could not decrypt ciphertext")
        })
        .collect::<Result<Vec<_>>>()?;

    InnerResponse { plaintext }.try_into()
}
