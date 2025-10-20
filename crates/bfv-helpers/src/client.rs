// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::build_bfv_params_arc;
use anyhow::{anyhow, Result};
use fhe_rs::bfv::{Encoding, Plaintext, PublicKey};
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use greco::vectors::GrecoVectors;
use num_bigint::BigInt;
use num_traits::Num;
use rand::thread_rng;

/// Encrypt a u64 using BFV homomorphic encryption
///
/// # Arguments
/// * `data` - The value to encrypt (u64)
/// * `public_key` - Serialized BFV public key bytes
/// # `degree` - Polynomial degree for BFV parameters
/// # `plaintext_modulus` - Plaintext modulus for BFV parameters
/// * `moduli` - Vector of moduli for BFV parameters
///
/// # Returns
/// * `Result<Vec<u8>>` - Serialized BFV ciphertext bytes
///
/// # Errors
/// Returns error string if:
/// - Public key deserialization fails
/// - Plaintext encoding fails  
/// - Encryption fails
/// - Input validation vector computation fails
pub fn bfv_encrypt_u64(
    data: u64,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: [u64; 1],
) -> Result<Vec<u8>> {
    let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

    let pk = PublicKey::from_bytes(&public_key, &params)
        .map_err(|e| anyhow!("Error deserializing public key:{e}"))?;

    let input = vec![data];
    let pt = Plaintext::try_encode(&input, Encoding::poly(), &params)
        .map_err(|e| anyhow!("Error encoding plaintext: {e}"))?;

    let ct = pk
        .try_encrypt(&pt, &mut thread_rng())
        .map_err(|e| anyhow!("Error encrypting data: {e}"))?;

    let encrypted_data = ct.to_bytes();
    Ok(encrypted_data)
}

#[derive(Debug, Clone)]
pub struct VerifiableEncryptionResult {
    pub encrypted_vote: Vec<u8>,
    pub circuit_inputs: String,
}

/// Verifiably encrypt a u64 using BFV homomorphic encryption and generate circuit inputs
/// to pass into Greco to prove the validity of the ciphertext
///
/// # Arguments
/// * `data` - The value to encrypt (u64)
/// * `public_key` - Serialized BFV public key bytes
/// # `degree` - Polynomial degree for BFV parameters
/// # `plaintext_modulus` - Plaintext modulus for BFV parameters
/// * `moduli` - Vector of moduli for BFV parameters
///
/// # Returns
/// * `Result<VerifiableEncryptionResult, String>` - Contains encrypted u64 and circuit inputs for ZKP
///
/// # Errors
/// Returns error string if:
/// - Public key deserialization fails
/// - Plaintext encoding fails  
/// - Encryption fails
/// - Input validation vector computation fails
pub fn bfv_verifiable_encrypt_u64(
    vote: u64,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: [u64; 1],
) -> Result<VerifiableEncryptionResult> {
    let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

    let pk = PublicKey::from_bytes(&public_key, &params)
        .map_err(|e| anyhow!("Error deserializing public key: {}", e))?;

    let vote_vector = vec![vote];

    let plaintext = Plaintext::try_encode(&vote_vector, Encoding::poly(), &params)
        .map_err(|e| anyhow!("Error encoding plaintext: {}", e))?;

    let (cipher_text, u_rns, e0_rns, e1_rns) = pk
        .try_encrypt_extended(&plaintext, &mut thread_rng())
        .map_err(|e| anyhow!("Error encrypting vote: {}", e))?;

    // Generate bounds and vectors directly
    // let (crypto_params, bounds) = GrecoBounds::compute(bfv_params, 0)?;

    // Create Greco input validation ZK proof
    let input_val_vectors = GrecoVectors::compute(
        &plaintext,
        &u_rns,
        &e0_rns,
        &e1_rns,
        &cipher_text,
        &pk,
        &params,
    )
    .map_err(|e| anyhow!("Error computing input validation vectors: {}", e))?;

    let zkp_modulus = BigInt::from_str_radix(
        "21888242871839275222246405745257275088548364400416034343698204186575808495617",
        10,
    )
    .unwrap();

    let standard_input_val = input_val_vectors.standard_form(&zkp_modulus);

    Ok(VerifiableEncryptionResult {
        encrypted_vote: cipher_text.to_bytes(),
        circuit_inputs: standard_input_val.to_json().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use crate::params::SET_2048_1032193_1;

    use super::*;

    #[test]
    fn test_bfv_encrypt_u64() {
        use fhe_rs::bfv::{Ciphertext, PublicKey, SecretKey};
        use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};

        let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let num = 1;
        let encrypted_vote =
            bfv_encrypt_u64(num, pk.to_bytes(), degree, plaintext_modulus, moduli).unwrap();

        let ct = Ciphertext::from_bytes(&encrypted_vote, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], num);
    }

    #[test]
    fn test_bfv_verifiable_encrypt_u64() {
        use fhe_rs::bfv::{Ciphertext, PublicKey, SecretKey};
        use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};

        let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let num = 1;
        let encrypted_vote =
            bfv_verifiable_encrypt_u64(num, pk.to_bytes(), degree, plaintext_modulus, moduli)
                .unwrap();

        let ct = Ciphertext::from_bytes(&encrypted_vote.encrypted_vote, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], num);
    }
}
