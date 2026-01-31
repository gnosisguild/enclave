// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_fhe_params::default_param_set;

// This could eventually be set here with an environment var once we allow for dynamic circuit selection.
pub fn get_default_paramset() -> e3_fhe_params::BfvParamSet {
    // NOTE: parameters are insecure. These parameters are mainly for testing and demonstration
    default_param_set()
}
