use e3_sdk::bfv_helpers::{BfvParamSet, BfvParams};

// This could eventually be set here with an environment var once we allow for dynamic circuit selection.
pub fn get_default_paramset() -> BfvParamSet {
    // NOTE: parameters are insecure. These parameters are mainly for testing and demonstration
    BfvParams::InsecureSet512_10_1.into()
}
