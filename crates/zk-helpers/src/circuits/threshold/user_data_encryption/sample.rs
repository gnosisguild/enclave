// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for user data encryption circuit.
//!
//! [`Sample`] produces a random BFV key pair and plaintext; the public key and plaintext are used as input
//! for codegen and tests.

use fhe::bfv::{BfvParameters, Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::FheEncoder;
use rand::thread_rng;
use std::sync::Arc;

/// A sample BFV public key and plaintext for user data encryption circuit codegen or tests.
#[derive(Debug, Clone)]
pub struct Sample {
    pub public_key: PublicKey,
    pub plaintext: Plaintext,
}

impl Sample {
    /// Generates a random secret key, public key, and plaintext for the given BFV parameters.
    pub fn generate(params: &Arc<BfvParameters>) -> Self {
        let mut rng = thread_rng();

        let secret_key = SecretKey::random(&params, &mut rng);
        let public_key = PublicKey::new(&secret_key, &mut rng);

        let plaintext = Plaintext::try_encode(&[1u64], Encoding::poly(), &params).unwrap();

        Self {
            public_key,
            plaintext,
        }
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
        assert_eq!(sample.plaintext.value.len(), params.degree());
    }
}
