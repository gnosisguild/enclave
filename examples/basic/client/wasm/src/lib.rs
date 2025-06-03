use e3_bfv_helpers::{build_bfv_params_arc, params::SET_2048_1032193_1};
use fhe_rs::bfv::{Encoding, Plaintext, PublicKey};
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use rand::thread_rng;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Encrypt {
    encrypted_vote: Vec<u8>,
}

#[wasm_bindgen]
impl Encrypt {
    pub fn encrypt_vote(&mut self, vote: u64, public_key: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

        let pk = PublicKey::from_bytes(&public_key, &params)
            .map_err(|e| JsValue::from_str(&format!("Error deserializing public key: {}", e)))?;

        let votes = vec![vote];
        let pt = Plaintext::try_encode(&votes, Encoding::poly(), &params)
            .map_err(|e| JsValue::from_str(&format!("Error encoding plaintext: {}", e)))?;

        let ct = pk
            .try_encrypt(&pt, &mut thread_rng())
            .map_err(|e| JsValue::from_str(&format!("Error encrypting vote: {}", e)))?;

        self.encrypted_vote = ct.to_bytes();
        Ok(self.encrypted_vote.clone())
    }
}
