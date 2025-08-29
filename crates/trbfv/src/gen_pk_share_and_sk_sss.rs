// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{ArcBytes, SharedRng, TrBFVConfig};
use anyhow::Result;
/// This module defines event payloads that will generate the public key share as well as the sk shamir secret shares to be distributed to other members of the committee.
/// This has been separated from the esi setup in order to be able to take advantage of parallelism
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_rs::{
    bfv::SecretKey,
    mbfv::{CommonRandomPoly, PublicKeyShare},
    trbfv::ShareManager,
};
use fhe_traits::Serialize as FheSerialize;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// Crp
    pub crp: ArcBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// PublicKey share for this node
    pub pk_share: ArcBytes,
    /// SecretKey Shamir Shares for other parties
    pub sk_sss: Vec<SensitiveBytes>,
}

pub async fn gen_pk_share_and_sk_sss(
    rng: &SharedRng,
    cipher: &Cipher,
    req: Request,
) -> Result<Response> {
    let params = req.trbfv_config.params();
    let crp = CommonRandomPoly::deserialize(&req.crp, &params)?;
    let threshold = req.trbfv_config.threshold();
    let num_ciphernodes = req.trbfv_config.num_parties();

    let sk_share = { SecretKey::random(&params, &mut *rng.lock().unwrap()) };
    let pk_share = { PublicKeyShare::new(&sk_share, crp.clone(), &mut *rng.lock().unwrap())? };

    let mut share_manager =
        ShareManager::new(num_ciphernodes as usize, threshold as usize, params.clone());

    let sk_poly = share_manager.coeffs_to_poly_level0(sk_share.coeffs.clone().as_ref())?;

    // has length of moduli
    // each entry holds num ciphernodes rows
    let sk_sss = share_manager.generate_secret_shares_from_poly(sk_poly)?;

    let sk_sss_result = sk_sss
        .into_iter()
        .map(|s| SensitiveBytes::new(bincode::serialize(&s)?, &cipher))
        .collect::<Result<_>>();

    Ok(Response {
        pk_share: Arc::new(pk_share.to_bytes()),
        sk_sss: sk_sss_result?,
    })
}
