// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub use e3_fhe_params::{BfvParamSet, BfvPreset};

/// Default BFV preset for the whole codebase.
#[cfg(debug_assertions)]
pub const DEFAULT_BFV_PRESET: BfvPreset = BfvPreset::InsecureThreshold512;
