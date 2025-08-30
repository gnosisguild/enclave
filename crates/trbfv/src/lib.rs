// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod calculate_decryption_key;
pub mod calculate_decryption_share;
pub mod calculate_threshold_decryption;
pub mod gen_esi_sss;
pub mod gen_pk_share_and_sk_sss;
pub mod helpers;
pub mod trbfv_config;
pub mod trbfv_request;
pub use trbfv_request::*;

use rand_chacha::ChaCha20Rng;
use std::sync::{Arc, Mutex};

pub use trbfv_config::*;

pub type ArcBytes = Arc<Vec<u8>>;
pub type SharedRng = Arc<Mutex<ChaCha20Rng>>;
/// Semantic PartyId
pub type PartyId = u64;
