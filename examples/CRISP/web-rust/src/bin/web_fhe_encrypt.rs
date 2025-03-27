mod greco;
mod util;

use console_log;
use greco::greco::*;
use log::info; // Use log macros from the `log` crate
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*; // For setting up logging to the browser console

use serde::Deserialize;
use std::{env, sync::Arc, thread, time};

use fhe::{
    bfv::{BfvParametersBuilder, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
    proto::bfv::Parameters,
};
use fhe_traits::{
    DeserializeParametrized, FheDecoder, FheDecrypter, FheEncoder, FheEncrypter, Serialize,
};
use rand::{distributions::Uniform, prelude::Distribution, rngs::OsRng, thread_rng, SeedableRng};
use util::timeit::{timeit, timeit_n};

#[wasm_bindgen]
pub struct Encrypt {
    encrypted_vote: Vec<u8>,
}

#[wasm_bindgen]
impl Encrypt {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Encrypt {
        Encrypt {
            encrypted_vote: Vec::new(),
        }
    }

    pub fn encrypt_vote(&mut self, vote: u64, public_key: Vec<u8>) -> Result<Vec<u8>, JsValue> {
        let degree = 2048;
        let plaintext_modulus: u64 = 1032193;
        let moduli = vec![0xffffffff00001];

        let params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(&moduli)
            .build_arc()
            .map_err(|e| JsValue::from_str(&format!("Error generating parameters: {}", e)))?;

        let pk = PublicKey::from_bytes(&public_key, &params)
            .map_err(|e| JsValue::from_str(&format!("Error deserializing public key: {}", e)))?;

        let votes = vec![vote];
        let pt = Plaintext::try_encode(&votes, Encoding::poly(), &params)
            .map_err(|e| JsValue::from_str(&format!("Error encoding plaintext: {}", e)))?;

        let (ct, u_rns, e0_rns, e1_rns) = pk
            .try_encrypt_extended(&pt, &mut thread_rng())
            .map_err(|e| JsValue::from_str(&format!("Error encrypting vote: {}", e)))?;

        // Create Greco input validation ZKP proof
        // let input_val_vectors =
        //     InputValidationVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk).map_err(
        //         |e| JsValue::from_str(&format!("Error computing input validation vectors: {}", e)),
        //     )?;

        self.encrypted_vote = ct.to_bytes();
        Ok(self.encrypted_vote.clone())
    }

    pub fn test() {
        web_sys::console::log_1(&"Test Function Working".into());
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}

// Tests
#[wasm_bindgen_test]
fn test_encrypt_vote() {
    // Initialize the logger to print to the browser's console
    console_log::init_with_level(log::Level::Info).expect("Error initializing logger");

    let degree = 2048;
    let plaintext_modulus: u64 = 1032193; // Must be Co-prime with Q
    let moduli = vec![0xffffffff00001]; // Must be 52-bit or less

    let params = BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(&moduli)
        .build_arc()
        .unwrap();

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
