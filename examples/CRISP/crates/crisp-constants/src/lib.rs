// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_sdk::bfv_helpers::{BfvParamSet, BfvPreset};

// This could eventually be set here with an environment var once we allow for dynamic circuit selection.
pub fn get_default_paramset() -> BfvParamSet {
    // NOTE: parameters are insecure. These parameters are mainly for testing and demonstration
    BfvPreset::InsecureThresholdBfv512.into()
}
