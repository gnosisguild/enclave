// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod ext;
mod runtime;

pub use ext::{FheExtension, FheRepositoryFactory, FHE_KEY};
pub use runtime::*;

// Re-export params so dependents can use e3_fhe::BfvPreset etc. without depending on e3-fhe-params.
pub use e3_fhe_params::{
    build_bfv_params, build_bfv_params_arc, build_bfv_params_from_set,
    build_bfv_params_from_set_arc, build_pair_for_preset, default_param_set, BfvParamSet,
    BfvPreset, ParameterType, PresetError, PresetMetadata, PresetSearchDefaults, SecurityTier,
    DEFAULT_BFV_PRESET,
};
pub use e3_fhe_params::{
    create_crp, create_deterministic_crp_from_default_seed, create_deterministic_crp_from_seed,
    setup_crp_params, ParamsWithCrp,
};
pub use e3_fhe_params::{
    decode_bfv_params, decode_bfv_params_arc, encode_bfv_params, EncodingError,
};
