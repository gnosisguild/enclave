use fhe::bfv::Ciphertext;
use fhe_traits::{DeserializeParametrized, Serialize};
use commons::bfv::deserialize_bfv_params_arc;
use compute_provider::FHEInputs;

/// CRISP Implementation of the CiphertextProcessor function
pub fn fhe_processor(fhe_inputs: &FHEInputs) -> Vec<u8> {
    let params = deserialize_bfv_params_arc(&fhe_inputs.params);

    let mut sum = Ciphertext::zero(&params);
    for ciphertext_bytes in &fhe_inputs.ciphertexts {
        let ciphertext = Ciphertext::from_bytes(&ciphertext_bytes.0, &params).unwrap();
        sum += &ciphertext;
    }

    sum.to_bytes()
}