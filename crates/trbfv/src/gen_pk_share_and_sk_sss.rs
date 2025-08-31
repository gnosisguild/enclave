// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// This module defines event payloads that will generate the public key share as well as the sk shamir secret shares to be distributed to other members of the committee.
/// This has been separated from the esi setup in order to be able to take advantage of parallelism
use crate::{ArcBytes, SharedRng, TrBFVConfig};
use anyhow::Result;
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_rs::{
    bfv::SecretKey,
    mbfv::{CommonRandomPoly, PublicKeyShare},
    trbfv::ShareManager,
};
use fhe_traits::Serialize as FheSerialize;
use ndarray::Array2;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// Crp
    pub crp: ArcBytes,
}

struct InnerRequest {
    pub trbfv_config: TrBFVConfig,
    pub crp: CommonRandomPoly,
}

impl TryFrom<Request> for InnerRequest {
    type Error = anyhow::Error;

    fn try_from(value: Request) -> std::result::Result<Self, Self::Error> {
        let crp = CommonRandomPoly::deserialize(&value.crp, &value.trbfv_config.params())?;
        Ok(InnerRequest {
            trbfv_config: value.trbfv_config,
            crp,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// PublicKey share for this node
    pub pk_share: ArcBytes,
    /// SecretKey Shamir Shares for other parties
    pub sk_sss: Vec<SensitiveBytes>,
}

impl TryFrom<(InnerResponse, &Cipher)> for Response {
    type Error = anyhow::Error;
    fn try_from(
        (value, cipher): (InnerResponse, &Cipher),
    ) -> std::result::Result<Self, Self::Error> {
        let pk_share = Arc::new(value.pk_share.to_bytes());
        let sk_sss = SensitiveBytes::try_from_unserialized_vec(value.sk_sss, cipher)?;
        Ok(Response { pk_share, sk_sss })
    }
}

struct InnerResponse {
    /// PublicKey share for this node
    pub pk_share: PublicKeyShare,
    /// SecretKey Shamir Shares for other parties
    pub sk_sss: Vec<Array2<u64>>,
}

pub async fn gen_pk_share_and_sk_sss(
    rng: &SharedRng,
    cipher: &Cipher,
    req: Request,
) -> Result<Response> {
    let req = InnerRequest::try_from(req)?;

    let params = req.trbfv_config.params();
    let crp = req.crp;
    let threshold = req.trbfv_config.threshold();
    let num_ciphernodes = req.trbfv_config.num_parties();

    let sk_share = { SecretKey::random(&params, &mut *rng.lock().unwrap()) };
    let pk_share = { PublicKeyShare::new(&sk_share, crp.clone(), &mut *rng.lock().unwrap())? };

    let mut share_manager =
        ShareManager::new(num_ciphernodes as usize, threshold as usize, params.clone());

    let sk_poly = share_manager.coeffs_to_poly_level0(sk_share.coeffs.clone().as_ref())?;

    let sk_sss =
        { share_manager.generate_secret_shares_from_poly(sk_poly, &mut *rng.lock().unwrap())? };

    Ok(Response::try_from((
        InnerResponse { pk_share, sk_sss },
        cipher,
    ))?)
}
