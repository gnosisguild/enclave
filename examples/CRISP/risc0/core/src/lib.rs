use fhe::bfv::{BfvParameters, Ciphertext};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use compute_provider::FHEInputs;
use std::sync::Arc;

/// CRISP Implementation of the CiphertextProcessor function
pub fn fhe_processor(fhe_inputs: &FHEInputs) -> Vec<u8> {
    let params = Arc::new(BfvParameters::try_deserialize(&fhe_inputs.params).unwrap());

    let mut sum = Ciphertext::zero(&params);
    for ciphertext_bytes in &fhe_inputs.ciphertexts {
        let ciphertext = Ciphertext::from_bytes(&ciphertext_bytes.0, &params).unwrap();
        sum += &ciphertext;
    }

    sum.to_bytes()
}