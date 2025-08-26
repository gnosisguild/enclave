// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// This module defines event payloads that will dcrypt a ciphertext with a threshold quorum of decryption shares
use crate::{ArcBytes, PartyId, TrBFVConfig};
use e3_crypto::Cipher;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// All decryption shares from a threshold quorum of nodes polys.
    pub d_share_polys: Vec<(PartyId, ArcBytes)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// The resultant plaintext
    pub plaintext: ArcBytes,
}

pub async fn calculate_threshold_decryption(cipher: &Cipher, req: Request) -> Response {
    todo!()
}
