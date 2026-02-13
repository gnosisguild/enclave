// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod helpers;
pub use helpers::*;

use std::path::PathBuf;

use num_bigint::{BigInt, Sign};

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

const FIELD_SIZE: usize = 32;

/// Extract a field element from public signals at the given index (0-based).
pub fn extract_field(signals: &[u8], index: usize) -> BigInt {
    let offset = index * FIELD_SIZE;
    BigInt::from_bytes_be(Sign::Plus, &signals[offset..offset + FIELD_SIZE])
}

/// Extract a field element from the end of public signals (0 = last, 1 = second to last, etc.)
pub fn extract_field_from_end(signals: &[u8], from_end: usize) -> BigInt {
    let total_fields = signals.len() / FIELD_SIZE;
    extract_field(signals, total_fields - 1 - from_end)
}
