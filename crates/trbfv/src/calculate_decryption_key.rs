// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{ArcBytes, TrBFVConfig};
/// This module defines event payloads that will generate the decryption key material to create a decryption share
use e3_crypto::{Cipher, SensitiveBytes};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// All collected secret key shamir shares
    pub sk_sss_collected: Vec<SensitiveBytes>,
    /// All collected smudging noise shamir shares
    pub esi_sss_collected: Vec<SensitiveBytes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
    /// A single summed polynomial for this partys smudging noise
    pub es_poly_sum: Vec<SensitiveBytes>,
}

pub async fn calculate_decryption_key(cipher: &Cipher, req: Request) -> Response {
    todo!()
}
