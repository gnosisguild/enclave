// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use anyhow::Result;
use e3_crypto::{Cipher, SensitiveBytes};
use fhe_math::rq::Poly;
use fhe_rs::bfv::BfvParameters;
use fhe_traits::DeserializeWithContext;

pub fn try_poly_from_bytes(bytes: &[u8], params: &BfvParameters) -> Result<Poly> {
    Ok(Poly::from_bytes(bytes, params.ctx_at_level(0)?)?)
}

pub fn try_poly_from_sensitive_bytes(
    bytes: SensitiveBytes,
    params: Arc<BfvParameters>,
    cipher: &Cipher,
) -> Result<Poly> {
    try_poly_from_bytes(&bytes.access(cipher)?, &params)
}

pub fn try_polys_from_sensitive_bytes_vec(
    bytes_vec: Vec<SensitiveBytes>,
    params: Arc<BfvParameters>,
    cipher: &Cipher,
) -> Result<Vec<Poly>> {
    bytes_vec
        .into_iter()
        .map(|s| try_poly_from_sensitive_bytes(s, params.clone(), cipher))
        .collect::<Result<Vec<_>>>()
}
