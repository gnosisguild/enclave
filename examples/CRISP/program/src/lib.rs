// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_bfv_helpers::{
    decode_bfv_params_arc,
    utils::greco::abi_decode_greco_to_bfv_bytes,
};
use e3_compute_provider::FHEInputs;
use fhe::bfv::Ciphertext;
use fhe_traits::{DeserializeParametrized, Serialize};

/// CRISP Implementation of the CiphertextProcessor function
pub fn fhe_processor(fhe_inputs: &FHEInputs) -> Vec<u8> {
    let params = decode_bfv_params_arc(&fhe_inputs.params);

    let mut sum = Ciphertext::zero(&params);
    for ciphertext_bytes in &fhe_inputs.ciphertexts {
        // Convert ABI-encoded greco ciphertext to BFV ciphertext bytes
        let bfv_bytes = abi_decode_greco_to_bfv_bytes(&ciphertext_bytes.0, &params)
            .expect("Failed to convert greco ciphertext to BFV");
        
        // Deserialize BFV ciphertext and add to sum
        let ciphertext = Ciphertext::from_bytes(&bfv_bytes, &params)
            .expect("Failed to deserialize BFV ciphertext");
        
        sum += &ciphertext;
    }

    sum.to_bytes()
}
