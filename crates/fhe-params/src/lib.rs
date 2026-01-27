// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Preset definitions and builders for zkFHE parameters.

pub mod builder;
pub mod constants;
#[cfg(feature = "abi-encoding")]
pub mod encoding;
pub mod presets;
pub mod search;

pub use builder::{
    build_bfv_params, build_bfv_params_arc, build_bfv_params_from_set,
    build_bfv_params_from_set_arc,
};
#[cfg(feature = "abi-encoding")]
pub use encoding::{decode_bfv_params, decode_bfv_params_arc, encode_bfv_params, EncodingError};
pub use presets::{
    BfvParamSet, BfvPreset, ParameterType, PresetError, PresetMetadata, PresetSearchDefaults,
};
