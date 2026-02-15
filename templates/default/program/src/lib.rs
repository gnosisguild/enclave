// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_compute_provider::FHEInputs;
use e3_fhe_params::decode_bfv_params_arc;
use fhe::bfv::Ciphertext;
use fhe_traits::{DeserializeParametrized, Serialize};

/// Implementation of the CiphertextProcessor function
pub fn fhe_processor(fhe_inputs: &FHEInputs) -> Vec<u8> {
    let params = decode_bfv_params_arc(&fhe_inputs.params).unwrap();

    let mut sum = Ciphertext::zero(&params);
    for ciphertext_bytes in &fhe_inputs.ciphertexts {
        let ciphertext = Ciphertext::from_bytes(&ciphertext_bytes.0, &params).unwrap();
        sum += &ciphertext;
    }

    sum.to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use e3_fhe_params::DEFAULT_BFV_PRESET;
    use e3_fhe_params::{BfvParamSet, build_bfv_params_arc, encode_bfv_params};
    use fhe::bfv::{Encoding, Plaintext, PublicKey, SecretKey};
    use fhe_traits::FheEncoder;
    use fhe_traits::FheEncrypter;
    use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};
    use rand::{rngs::OsRng, thread_rng};

    #[test]
    fn test() -> Result<()> {
        let mut rng = thread_rng();

        let params_set: BfvParamSet = DEFAULT_BFV_PRESET.into();
        let params = build_bfv_params_arc(
            params_set.degree,
            params_set.plaintext_modulus,
            &params_set.moduli,
            params_set.error1_variance,
        );

        let secret_key = SecretKey::random(&params, &mut OsRng);
        let public_key = PublicKey::new(&secret_key, &mut rng);

        // 3
        let three = public_key.try_encrypt(
            &Plaintext::try_encode(&[3u64], Encoding::poly(), &params)?,
            &mut rng,
        )?;

        // 2
        let two = public_key.try_encrypt(
            &Plaintext::try_encode(&[2u64], Encoding::poly(), &params)?,
            &mut rng,
        )?;

        // Prepare inputs
        let fhe_inputs = FHEInputs {
            params: encode_bfv_params(&params),
            ciphertexts: vec![(three.to_bytes(), 0), (two.to_bytes(), 1)],
        };

        // Run the processor
        let result = fhe_processor(&fhe_inputs);

        // Decrypt result
        let decrypted = secret_key.try_decrypt(&Ciphertext::from_bytes(&result, &params)?)?;

        assert_eq!(decrypted.value[0], 5);
        Ok(())
    }
}
