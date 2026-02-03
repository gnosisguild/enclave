// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use fhe::bfv::{BfvParameters, PublicKey, SecretKey};
use rand::thread_rng;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Sample {
    pub public_key: PublicKey,
}

impl Sample {
    pub fn generate(params: &Arc<BfvParameters>) -> Self {
        let mut rng = thread_rng();

        let secret_key = SecretKey::random(&params, &mut rng);
        let public_key = PublicKey::new(&secret_key, &mut rng);

        Self { public_key }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::BfvParamSet;
    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_generate_sample() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let sample = Sample::generate(&params);

        assert_eq!(sample.public_key.c.c.len(), 2);
    }
}
