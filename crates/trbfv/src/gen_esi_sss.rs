// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    shares::{Encrypted, SharedSecret},
    TrBFVConfig,
};
use anyhow::{bail, Result};
use e3_crypto::{Cipher, SensitiveBytes};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenEsiSssRequest {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// Pre-generated smudging noise polynomial (private witness, encrypted at rest).
    pub e_sm_raw: SensitiveBytes,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenEsiSssResponse {
    /// The smudging noise shares
    pub esi_sss: Vec<Encrypted<SharedSecret>>,
}

/// Smudging noise is sampled locally at decryption. This request no longer deals shares.
pub fn gen_esi_sss<R: RngCore + CryptoRng>(
    _rng: &mut R,
    _cipher: &Cipher,
    _req: GenEsiSssRequest,
) -> Result<GenEsiSssResponse> {
    bail!("smudging noise is sampled at decryption and is not Shamir-shared")
}
