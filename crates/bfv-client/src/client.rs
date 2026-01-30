// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{anyhow, Result};
use e3_fhe_params::build_bfv_params_arc;
use e3_greco_helpers::{bfv_ciphertext_to_greco, bfv_public_key_to_greco};
use e3_zk_helpers::commitments::{
    compute_ciphertext_commitment, compute_threshold_pk_aggregation_commitment,
};
use e3_zk_helpers::utils::calculate_bit_width;
use fhe::bfv::{Ciphertext, Encoding, Plaintext, PublicKey};
use fhe::Error as FheError;
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use greco::bounds::GrecoBounds;
use greco::vectors::GrecoVectors;
use rand::thread_rng;

/// Encrypt some data using BFV homomorphic encryption
///
/// # Arguments
/// * `data` - The value to encrypt (Generic type T)
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
pub fn bfv_encrypt<T>(
    data: T,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: &[u64],
) -> Result<Vec<u8>>
where
    Plaintext: for<'a> FheEncoder<&'a T, Error = FheError>,
{
    let params = build_bfv_params_arc(degree, plaintext_modulus, moduli, None);

    let pk = PublicKey::from_bytes(&public_key, &params)
        .map_err(|e| anyhow!("Error deserializing public key:{e}"))?;

    let pt = Plaintext::try_encode(&data, Encoding::poly(), &params)
        .map_err(|e: FheError| anyhow!("Error encoding plaintext: {e}"))?;

    let ct = pk
        .try_encrypt(&pt, &mut thread_rng())
        .map_err(|e| anyhow!("Error encrypting data: {e}"))?;

    let encrypted_data = ct.to_bytes();
    Ok(encrypted_data)
}

#[derive(Debug, Clone)]
pub struct VerifiableEncryptionResult {
    pub encrypted_data: Vec<u8>,
    pub circuit_inputs: String,
}

/// Verifiably encrypt some data using BFV homomorphic encryption and generate circuit inputs
/// to pass into Greco to prove the validity of the ciphertext
///
/// # Arguments
/// * `data` - The value to encrypt (Generic type T)
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
pub fn bfv_verifiable_encrypt<T>(
    data: T,
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<VerifiableEncryptionResult>
where
    Plaintext: for<'a> FheEncoder<&'a T, Error = FheError>,
{
    let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli, None);

    let pk = PublicKey::from_bytes(&public_key, &params)
        .map_err(|e| anyhow!("Error deserializing public key: {}", e))?;

    let plaintext = Plaintext::try_encode(&data, Encoding::poly(), &params)
        .map_err(|e: FheError| anyhow!("Error encoding plaintext: {}", e))?;

    let (cipher_text, u_rns, e0_rns, e1_rns) = pk
        .try_encrypt_extended(&plaintext, &mut thread_rng())
        .map_err(|e| anyhow!("Error encrypting data: {}", e))?;

    let (_, bounds) = GrecoBounds::compute(&params, 0)?;

    let bit_pk = calculate_bit_width(&bounds.pk_bounds[0].to_string())?;

    // Create Greco input validation ZK proof
    let input_val_vectors = GrecoVectors::compute(
        &plaintext,
        &u_rns,
        &e0_rns,
        &e1_rns,
        &cipher_text,
        &pk,
        &params,
        bit_pk,
    )
    .map_err(|e| anyhow!("Error computing input validation vectors: {}", e))?;

    let standard_input_val = input_val_vectors.standard_form();

    Ok(VerifiableEncryptionResult {
        encrypted_data: cipher_text.to_bytes(),
        circuit_inputs: standard_input_val.to_json().to_string(),
    })
}

pub fn compute_pk_commitment(
    public_key: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<[u8; 32]> {
    let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli, None);

    let public_key = PublicKey::from_bytes(&public_key, &params)
        .map_err(|e| anyhow!("Error deserializing public key: {}", e))?;

    let (_, bounds) = GrecoBounds::compute(&params, 0)?;
    let bit_pk = calculate_bit_width(&bounds.pk_bounds[0].to_string())?;

    let (pk0is, pk1is) = bfv_public_key_to_greco(&public_key, &params);
    let commitment_bigint = compute_threshold_pk_aggregation_commitment(&pk0is, &pk1is, bit_pk);

    let bytes = commitment_bigint.to_bytes_be().1;

    if bytes.len() > 32 {
        return Err(anyhow!(
            "Commitment must be at most 32 bytes, got {}",
            bytes.len()
        ));
    }

    let mut padded_bytes = vec![0u8; 32];
    let start_idx = 32 - bytes.len();
    padded_bytes[start_idx..].copy_from_slice(&bytes);

    let public_key_hash: [u8; 32] = padded_bytes
        .try_into()
        .map_err(|_| anyhow!("Failed to convert padded bytes to array"))?;

    Ok(public_key_hash)
}

pub fn compute_ct_commitment(
    ct: Vec<u8>,
    degree: usize,
    plaintext_modulus: u64,
    moduli: Vec<u64>,
) -> Result<[u8; 32]> {
    let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli, None);

    let ct = Ciphertext::from_bytes(&ct, &params)
        .map_err(|e| anyhow!("Error deserializing ciphertext: {}", e))?;

    let (ct0is, ct1is) = bfv_ciphertext_to_greco(&ct, &params);

    let (_, bounds) = GrecoBounds::compute(&params, 0)?;
    let bit_ct = calculate_bit_width(&bounds.pk_bounds[0].to_string())?;

    let commitment_bigint = compute_ciphertext_commitment(&ct0is, &ct1is, bit_ct);

    let bytes = commitment_bigint.to_bytes_be().1;

    if bytes.len() > 32 {
        return Err(anyhow!(
            "Commitment must be at most 32 bytes, got {}",
            bytes.len()
        ));
    }

    let mut padded_bytes = vec![0u8; 32];
    let start_idx = 32 - bytes.len();
    padded_bytes[start_idx..].copy_from_slice(&bytes);

    let ciphertext_hash: [u8; 32] = padded_bytes
        .try_into()
        .map_err(|_| anyhow!("Failed to convert padded bytes to array"))?;

    Ok(ciphertext_hash)
}

#[cfg(test)]
mod tests {
    use e3_fhe_params::DEFAULT_BFV_PRESET;
    use e3_fhe_params::{build_bfv_params_from_set_arc, BfvParamSet};

    use super::*;

    #[test]
    fn test_bfv_encrypt_a64() {
        use fhe::bfv::{Ciphertext, PublicKey, SecretKey};
        use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};

        let param_set: BfvParamSet = DEFAULT_BFV_PRESET.into();
        let params = build_bfv_params_from_set_arc(param_set);
        let degree = param_set.degree;
        let plaintext_modulus = param_set.plaintext_modulus;
        let moduli = param_set.moduli;
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let num = [1u64];
        let encrypted_data =
            bfv_encrypt(num, pk.to_bytes(), degree, plaintext_modulus, &moduli).unwrap();

        let ct = Ciphertext::from_bytes(&encrypted_data, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], num[0]);
    }

    #[test]
    fn test_bfv_encrypt_v64() {
        use fhe::bfv::{Ciphertext, PublicKey, SecretKey};
        use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};

        let param_set: BfvParamSet = DEFAULT_BFV_PRESET.into();
        let params = build_bfv_params_from_set_arc(param_set);
        let degree = param_set.degree;
        let plaintext_modulus = param_set.plaintext_modulus;
        let moduli = param_set.moduli;
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let num = vec![1, 2];
        let encrypted_data = bfv_encrypt(
            num.clone(),
            pk.to_bytes(),
            degree,
            plaintext_modulus,
            &moduli,
        )
        .unwrap();

        let ct = Ciphertext::from_bytes(&encrypted_data, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], num[0]);
        assert_eq!(pt.value[1], num[1]);
    }

    #[test]
    fn test_bfv_verifiable_encrypt_a64() {
        use fhe::bfv::{Ciphertext, PublicKey, SecretKey};
        use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};

        let param_set: BfvParamSet = DEFAULT_BFV_PRESET.into();
        let params = build_bfv_params_from_set_arc(param_set);
        let degree = param_set.degree;
        let plaintext_modulus = param_set.plaintext_modulus;
        let moduli = param_set.moduli;
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let num = [1u64];
        let encrypted_data = bfv_verifiable_encrypt(
            num,
            pk.to_bytes(),
            degree,
            plaintext_modulus,
            moduli.to_vec(),
        )
        .unwrap();

        let ct = Ciphertext::from_bytes(&encrypted_data.encrypted_data, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], num[0]);
    }

    #[test]
    fn test_bfv_verifiable_encrypt_v64() {
        use fhe::bfv::{Ciphertext, PublicKey, SecretKey};
        use fhe_traits::{DeserializeParametrized, FheDecrypter, Serialize};

        let param_set: BfvParamSet = DEFAULT_BFV_PRESET.into();
        let params = build_bfv_params_from_set_arc(param_set);
        let degree = param_set.degree;
        let plaintext_modulus = param_set.plaintext_modulus;
        let moduli = param_set.moduli;
        let mut rng = thread_rng();
        let sk = SecretKey::random(&params, &mut rng);
        let pk = PublicKey::new(&sk, &mut rng);

        let num = vec![1, 2];
        let encrypted_data = bfv_verifiable_encrypt(
            num.clone(),
            pk.to_bytes(),
            degree,
            plaintext_modulus,
            moduli.to_vec(),
        )
        .unwrap();

        let ct = Ciphertext::from_bytes(&encrypted_data.encrypted_data, &params).unwrap();
        let pt = sk.try_decrypt(&ct).unwrap();

        assert_eq!(pt.value[0], num[0]);
        assert_eq!(pt.value[1], num[1]);
    }
}
