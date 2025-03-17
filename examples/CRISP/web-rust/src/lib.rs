mod greco;
mod util;

use console_log;
use greco::{greco::*, pk_encryption_circuit::create_pk_enc_proof};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*; // For setting up logging to the browser console
pub use wasm_bindgen_rayon::init_thread_pool;

use fhe::bfv::{BfvParametersBuilder, Ciphertext, Encoding, Plaintext, PublicKey, SecretKey};
use fhe_traits::{DeserializeParametrized, FheDecrypter, FheEncoder, Serialize};
use num_bigint::BigInt;
use num_traits::Num;
use rand::thread_rng;
use js_sys::{Uint8Array, Array};

#[wasm_bindgen]
pub struct Encrypt {
    vote: Vec<u8>,
    proof: Vec<u8>,
    instances: Vec<Vec<u8>>,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct EncryptionResult {
    encrypted_vote: Vec<u8>,
    proof: Vec<u8>,
    instances: Vec<Vec<u8>>,
}


#[wasm_bindgen]
impl EncryptionResult {
    #[wasm_bindgen(getter)]
    pub fn encrypted_vote(&self) -> Vec<u8> {
        self.encrypted_vote.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn instances(&self) -> Array {
        let instances = Array::new();
        for instance in self.instances.iter() {
            instances.push(&Uint8Array::from(instance.as_slice()));
        }
        instances
    }
}

#[wasm_bindgen]
impl Encrypt {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Encrypt {
        Encrypt {
            vote: Vec::new(),
            proof: Vec::new(),
            instances: Vec::new(),
        }
    }
    pub fn encrypt_vote(&mut self, vote: u64, public_key: Vec<u8>) -> Result<EncryptionResult, JsValue> {
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
        let input_val_vectors =
            InputValidationVectors::compute(&pt, &u_rns, &e0_rns, &e1_rns, &ct, &pk).map_err(
                |e| JsValue::from_str(&format!("Error computing input validation vectors: {}", e)),
            )?;

        // Initialize zk proving modulus
        let p = BigInt::from_str_radix(
            "21888242871839275222246405745257275088548364400416034343698204186575808495617",
            10,
        )
        .unwrap();

        let standard_input_val = input_val_vectors.standard_form(&p);

        let (proof, instances) = create_pk_enc_proof(standard_input_val);
        self.proof = proof;
        self.instances = instances.clone();
        self.vote = ct.to_bytes();
        Ok(EncryptionResult {
            encrypted_vote: self.vote.clone(),
            proof: self.proof.clone(),
            instances: instances.clone(),
        })
    }
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

    let ct = Ciphertext::from_bytes(&test.vote, &params).unwrap();
    let pt = sk.try_decrypt(&ct).unwrap();

    assert_eq!(pt.value[0], vote);
}
