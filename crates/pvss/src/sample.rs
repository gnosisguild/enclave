// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::types::Sample;
use fhe::bfv::{BfvParameters, PublicKey, SecretKey};
use rand::thread_rng;
use std::sync::Arc;

pub fn generate_sample(params: &Arc<BfvParameters>) -> Sample {
    let mut rng = thread_rng();

    let secret_key = SecretKey::random(&params, &mut rng);
    let public_key = PublicKey::new(&secret_key, &mut rng);

    Sample { public_key }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::{BfvParamSet, BfvPreset};

    #[test]
    fn test_generate_sample() {
        let params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let sample = generate_sample(&params);

        assert_eq!(sample.public_key.c.c.len(), 2);
    }
}
