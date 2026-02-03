// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for circuits.
//!
//! [`Sample`] produces a random BFV key pair; the public key is used as input
//! for codegen and tests (e.g. pk-bfv circuit).

use fhe::bfv::{BfvParameters, PublicKey, SecretKey};
use rand::thread_rng;
use std::sync::Arc;

/// A sample BFV public key (and optionally related data) for circuit codegen or tests.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Randomly generated BFV public key.
    pub public_key: PublicKey,
}

impl Sample {
    /// Generates a random secret key and public key for the given BFV parameters.
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
