// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_bfv_helpers::{
    decode_bfv_params_arc,
    utils::greco::{abi_decode_greco_ciphertext, greco_to_bfv_ciphertext},
};
use e3_compute_provider::FHEInputs;
use fhe::bfv::Ciphertext;
use fhe_traits::Serialize;

/// CRISP Implementation of the CiphertextProcessor function
pub fn fhe_processor(fhe_inputs: &FHEInputs) -> Vec<u8> {
    let params = decode_bfv_params_arc(&fhe_inputs.params);

    let mut sum = Ciphertext::zero(&params);
    for ciphertext_bytes in &fhe_inputs.ciphertexts {
        let (ct0is, ct1is) = abi_decode_greco_ciphertext(&ciphertext_bytes.0, &params);
        let ciphertext = greco_to_bfv_ciphertext(&ct0is, &ct1is, &params);

        sum += &ciphertext;
    }

    sum.to_bytes()
}
