// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for user data encryption circuit.
//!
//! [`Sample`] produces a random BFV key pair and plaintext; the public key and plaintext are used as input
//! for codegen and tests.

use e3_fhe_params::{build_pair_for_preset, BfvPreset};
use fhe::bfv::{Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::FheEncoder;
use rand::thread_rng;

/// A sample BFV public key and plaintext for user data encryption circuit codegen or tests.
#[derive(Debug, Clone)]
pub struct UserDataEncryptionSample {
    pub public_key: PublicKey,
    pub plaintext: Plaintext,
}

impl UserDataEncryptionSample {
    /// Generates a random secret key, public key, and plaintext for the given BFV parameters.
    pub fn generate(preset: BfvPreset) -> Self {
        let (threshold_params, _) = build_pair_for_preset(preset).unwrap();

        let mut rng = thread_rng();

        let secret_key = SecretKey::random(&threshold_params, &mut rng);
        let public_key = PublicKey::new(&secret_key, &mut rng);

        let plaintext =
            Plaintext::try_encode(&[1u64], Encoding::poly(), &threshold_params).unwrap();

        Self {
            public_key,
            plaintext,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_generate_sample() {
        let sample = UserDataEncryptionSample::generate(DEFAULT_BFV_PRESET);

        assert_eq!(sample.public_key.c.c.len(), 2);
        assert_eq!(
            sample.plaintext.value.len(),
            DEFAULT_BFV_PRESET.metadata().degree
        );
    }
}
