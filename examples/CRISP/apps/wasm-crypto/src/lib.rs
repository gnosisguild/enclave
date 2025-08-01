// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod util;

use console_log;
use e3_bfv_helpers::{build_bfv_params_arc, params::SET_2048_1032193_1};
use fhe_rs::bfv::{Ciphertext, Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::{DeserializeParametrized, FheDecrypter, FheEncoder, Serialize};
use greco::InputValidationVectors;
use num_bigint::BigInt;
use num_traits::Num;
use rand::thread_rng;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*; // For setting up logging to the browser console

#[wasm_bindgen]
pub struct Encrypt {
    encrypted_vote: Vec<u8>,
}

#[wasm_bindgen]
pub struct EncryptedVote {
    encrypted_vote: Vec<u8>,
    circuit_inputs: String,
}

#[wasm_bindgen]
impl EncryptedVote {
    #[wasm_bindgen(getter)]
    pub fn encrypted_vote(&self) -> Vec<u8> {
        self.encrypted_vote.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn circuit_inputs(&self) -> String {
        self.circuit_inputs.clone()
    }
}

#[wasm_bindgen]
impl Encrypt {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Encrypt {
        Encrypt {
            encrypted_vote: Vec::new(),
        }
    }

    pub fn encrypt_vote(
        &mut self,
        vote: u64,
        public_key: Vec<u8>,
    ) -> Result<EncryptedVote, JsValue> {
        let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

        let pk = PublicKey::from_bytes(&public_key, &params)
            .map_err(|e| JsValue::from_str(&format!("Error deserializing public key: {}", e)))?;

        let votes = vec![vote];
        let pt = Plaintext::try_encode(&votes, Encoding::poly(), &params)
            .map_err(|e| JsValue::from_str(&format!("Error encoding plaintext: {}", e)))?;

        let (ct, u_rns, e0_rns, e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .map_err(|e| JsValue::from_str(&format!("Error encrypting vote: {}", e)))?;

        // Create Greco input validation ZKP proof
        let input_val_vectors =
            InputValidationVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk, &params)
                .map_err(|e| {
                    JsValue::from_str(&format!("Error computing input validation vectors: {}", e))
                })?;

        let zkp_modulus = BigInt::from_str_radix(
            "21888242871839275222246405745257275088548364400416034343698204186575808495617",
            10,
        )
        .unwrap();

        let standard_input_val = input_val_vectors.standard_form(&zkp_modulus);
        self.encrypted_vote = ct.to_bytes();

        Ok(EncryptedVote {
            encrypted_vote: self.encrypted_vote.clone(),
            circuit_inputs: standard_input_val.to_json().to_string(),
        })
    }

    pub fn test() {
        web_sys::console::log_1(&"Test Function Working".into());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_encrypt_vote() {
        // Initialize the logger to print to the browser's console
        console_log::init_with_level(log::Level::Info).expect("Error initializing logger");

        let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let mut test = Encrypt::new();
        let vote = 10;
        test.encrypt_vote(vote, pk.to_bytes()).unwrap();

        let ct = Ciphertext::from_bytes(&test.encrypted_vote, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], vote);
    }
}
