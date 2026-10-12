// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Derive operational l-BFV keys from accepted share bytes.

use anyhow::{bail, Result};
use e3_fhe_params::BfvPreset;
use e3_utils::ArcBytes;

/// Operational key material from one accepted contribution set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LbfvOperationalKeys {
    /// Canonical fhe.rs serialization of the complete l-component public key.
    pub public_key: ArcBytes,
    /// Canonical fhe.rs serialization of the matching relinearization key.
    pub relinearization_key: ArcBytes,
}

/// Validate one public-key contribution against a supported l-BFV preset.
pub fn validate_lbfv_public_key_share_bytes(_bytes: &[u8]) -> Result<()> {
    bail!("l-BFV public-key shares are not available on this fhe.rs branch")
}

/// Validate one relinearization-key contribution against a supported l-BFV preset.
pub fn validate_lbfv_relinearization_key_share_bytes(_bytes: &[u8]) -> Result<()> {
    bail!("l-BFV relinearization-key shares are not available on this fhe.rs branch")
}

/// Aggregate accepted public-key and RLK contributions in one canonical order.
///
/// The caller must perform signer, party-set, session, and commitment validation before calling
/// this adapter. Both serializations derive from the same ordered contribution set.
pub fn aggregate_lbfv_keys(
    _preset: BfvPreset,
    _public_key_share_bytes: &[ArcBytes],
    _rlk_share_bytes: &[ArcBytes],
) -> Result<LbfvOperationalKeys> {
    bail!("l-BFV key aggregation is not available on this fhe.rs branch")
}
