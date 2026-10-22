// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    shares::{DecryptVec, Encrypted, ShamirShare, ShamirShareSliceExt},
    TrBFVConfig,
};
use anyhow::Result;
/// This module defines event payloads that will generate the decryption key material to create a decryption share
use anyhow::*;
use e3_crypto::{Cipher, SensitiveBytes};
use fhe::trbfv::ShareManager;
use fhe_math::rq::{Poly, PowerBasis};
use fhe_traits::Serialize;
use ndarray::Array2;
use tracing::info;
use zeroize::Zeroizing;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateDecryptionKeyRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// All collected secret key shamir shares where SensitiveBytes is Vec<Array2<u64>>
    pub sk_sss_collected: Vec<Encrypted<ShamirShare>>,
}

struct InnerRequest {
    pub trbfv_config: TrBFVConfig,
    pub sk_sss_collected: Vec<ShamirShare>,
}

impl TryFrom<(&Cipher, CalculateDecryptionKeyRequest)> for InnerRequest {
    type Error = anyhow::Error;
    fn try_from(
        value: (&Cipher, CalculateDecryptionKeyRequest),
    ) -> std::result::Result<Self, Self::Error> {
        let cipher = value.0;
        let req = value.1;
        info!("Converting sk_sss to collected...");

        // convert to collected
        let sk_sss_collected = req.sk_sss_collected.decrypt(cipher)?;

        Ok(InnerRequest {
            sk_sss_collected,
            trbfv_config: req.trbfv_config,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CalculateDecryptionKeyResponse {
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
}

struct InnerResponse {
    pub sk_poly_sum: Poly<PowerBasis>,
}

impl TryFrom<(&Cipher, InnerResponse)> for CalculateDecryptionKeyResponse {
    type Error = anyhow::Error;
    fn try_from(value: (&Cipher, InnerResponse)) -> std::result::Result<Self, Self::Error> {
        let InnerResponse { sk_poly_sum } = value.1;

        let cipher = value.0;

        Ok(CalculateDecryptionKeyResponse {
            sk_poly_sum: SensitiveBytes::new(sk_poly_sum.to_bytes(), cipher)?,
        })
    }
}

pub fn deserialize_to_array2(value: Zeroizing<Vec<u8>>) -> Result<Array2<u64>> {
    bincode::deserialize(&value).context("Error deserializing ndarray")
}

pub fn serialize_from_array2(value: Array2<u64>) -> Result<Vec<u8>> {
    bincode::serialize(&value).context("Error serializing ndarray")
}

pub fn calculate_decryption_key(
    cipher: &Cipher,
    req: CalculateDecryptionKeyRequest,
) -> Result<CalculateDecryptionKeyResponse> {
    info!("Calculating decryption key...");

    let req: InnerRequest = (cipher, req).try_into()?;

    let params = req.trbfv_config.params();
    let threshold = req.trbfv_config.threshold() as usize;
    let num_ciphernodes = req.trbfv_config.num_parties() as usize;
    let share_manager = ShareManager::new(num_ciphernodes, threshold, params.clone())?;

    info!("Calculating sk_poly_sum...");
    let sk_poly_sum = aggregate_secret_key_to_poly(&share_manager, &req.sk_sss_collected)?;

    info!("Returning successful result! Encrypting for transit...");

    (cipher, InnerResponse { sk_poly_sum }).try_into()
}

/// Sum the collected secret-key shares into this party's decryption polynomial.
fn aggregate_secret_key_to_poly(
    manager: &ShareManager,
    shares: &[ShamirShare],
) -> Result<Poly<PowerBasis>> {
    let transport = shares.to_array_data();
    Ok(manager.aggregate_collected_shares(&transport)?)
}
