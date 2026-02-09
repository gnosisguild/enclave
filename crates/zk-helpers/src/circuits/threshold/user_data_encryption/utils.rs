// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::utils::{compute_pk_bit, get_zkp_modulus, ZkHelpersUtilsError};
use e3_polynomial::{CrtPolynomial, CrtPolynomialError};
use fhe::bfv::{BfvParameters, Ciphertext, PublicKey};

/// Converts a BFV ciphertext to Greco format.
///
/// Takes a BFV ciphertext and converts it to Greco format, returning ct0is and ct1is
/// as CRT polynomials.
///
/// # Arguments
/// * `params` - BFV parameters
/// * `ct` - BFV ciphertext
///
/// # Returns
/// A tuple of (ct0is, ct1is) where each is CrtPolynomial
///
/// # Errors
/// Returns [`CrtPolynomialError::ModuliLengthMismatch`] if `moduli.len() != self.limbs.len()`.
pub fn bfv_ciphertext_to_greco(
    params: &BfvParameters,
    ciphertext: &Ciphertext,
) -> Result<(CrtPolynomial, CrtPolynomial), CrtPolynomialError> {
    let moduli = params.moduli();
    let zkp_modulus = get_zkp_modulus();

    let mut ct0is = CrtPolynomial::from_fhe_polynomial(&ciphertext.c[0]);
    let mut ct1is = CrtPolynomial::from_fhe_polynomial(&ciphertext.c[1]);

    ct0is.reverse();
    ct1is.reverse();

    ct0is.center(&moduli)?;
    ct1is.center(&moduli)?;

    ct0is.reduce_uniform(&zkp_modulus);
    ct1is.reduce_uniform(&zkp_modulus);

    Ok((ct0is, ct1is))
}

/// Converts a BFV public key to Greco format.
///
/// Takes a BFV public key and converts it to Greco format, returning pk0is and pk1is
/// as CRT polynomials.
///
/// # Arguments
/// * `params` - BFV parameters
/// * `public_key` - BFV public key
///
/// # Returns
/// A tuple of (pk0is, pk1is) where each is CrtPolynomial
///
/// # Errors
/// Returns [`CrtPolynomialError::ModuliLengthMismatch`] if `moduli.len() != self.limbs.len()`.
pub fn bfv_public_key_to_greco(
    params: &BfvParameters,
    public_key: &PublicKey,
) -> Result<(CrtPolynomial, CrtPolynomial), CrtPolynomialError> {
    let moduli = params.moduli();
    let zkp_modulus = get_zkp_modulus();

    let mut pk0is = CrtPolynomial::from_fhe_polynomial(&public_key.c.c[0]);
    let mut pk1is = CrtPolynomial::from_fhe_polynomial(&public_key.c.c[1]);

    pk0is.reverse();
    pk1is.reverse();

    pk0is.center(&moduli)?;
    pk1is.center(&moduli)?;

    pk0is.reduce_uniform(&zkp_modulus);
    pk1is.reduce_uniform(&zkp_modulus);

    Ok((pk0is, pk1is))
}

/// Computes the commitment of the public key.
///
/// # Arguments
/// * `params` - BFV parameters
/// * `public_key` - BFV public key
///
/// # Returns
/// The commitment of the public key
///
/// # Errors
/// Returns [`ZkHelpersUtilsError::ConversionError`] if the conversion fails.
/// Returns [`ZkHelpersUtilsError::CommitmentTooLong`] if the commitment is too long.
pub fn compute_public_key_commitment(
    params: &BfvParameters,
    public_key: &PublicKey,
) -> Result<[u8; 32], ZkHelpersUtilsError> {
    use crate::commitments::compute_pk_aggregation_commitment;

    let (pk0is, pk1is) = bfv_public_key_to_greco(&params, &public_key).map_err(|e| {
        ZkHelpersUtilsError::ConversionError(format!(
            "Failed to convert public key to greco: {}",
            e
        ))
    })?;

    let pk_bit = compute_pk_bit(params);
    let commitment = compute_pk_aggregation_commitment(&pk0is, &pk1is, pk_bit);

    let bytes = commitment.to_bytes_be().1;

    if bytes.len() > 32 {
        return Err(ZkHelpersUtilsError::CommitmentTooLong(bytes.len()));
    }

    let mut padded_bytes = vec![0u8; 32];
    let start_idx = 32 - bytes.len();
    padded_bytes[start_idx..].copy_from_slice(&bytes);

    let public_key_hash: [u8; 32] = padded_bytes.try_into().map_err(|_| {
        ZkHelpersUtilsError::ConversionError("Failed to convert padded bytes to array".into())
    })?;

    Ok(public_key_hash)
}

/// Computes the commitment of the ciphertext.
///
/// # Arguments
/// * `params` - BFV parameters
/// * `ciphertext` - BFV ciphertext
///
/// # Returns
/// The commitment of the ciphertext
///
/// # Errors
/// Returns [`ZkHelpersUtilsError::ConversionError`] if the conversion fails.
/// Returns [`ZkHelpersUtilsError::CommitmentTooLong`] if the commitment is too long.
pub fn compute_ciphertext_commitment(
    params: &BfvParameters,
    ciphertext: &Ciphertext,
) -> Result<[u8; 32], ZkHelpersUtilsError> {
    use crate::commitments::compute_ciphertext_commitment;

    let (ct0is, ct1is) = bfv_ciphertext_to_greco(&params, &ciphertext).map_err(|e| {
        ZkHelpersUtilsError::ConversionError(format!(
            "Failed to convert ciphertext to greco: {}",
            e
        ))
    })?;

    let pk_bit = compute_pk_bit(params);
    let commitment = compute_ciphertext_commitment(&ct0is, &ct1is, pk_bit);

    let bytes = commitment.to_bytes_be().1;

    if bytes.len() > 32 {
        return Err(ZkHelpersUtilsError::CommitmentTooLong(bytes.len()));
    }

    let mut padded_bytes = vec![0u8; 32];
    let start_idx = 32 - bytes.len();
    padded_bytes[start_idx..].copy_from_slice(&bytes);

    let ciphertext_hash: [u8; 32] = padded_bytes.try_into().map_err(|_| {
        ZkHelpersUtilsError::ConversionError("Failed to convert padded bytes to array".into())
    })?;

    Ok(ciphertext_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuits::computation::Computation;
    use crate::threshold::user_data_encryption::computation::Witness;
    use crate::threshold::user_data_encryption::sample::UserDataEncryptionSample;
    use crate::threshold::user_data_encryption::UserDataEncryptionCircuitInput;
    use e3_fhe_params::{build_pair_for_preset, BfvPreset};
    use fhe_traits::DeserializeParametrized;

    #[test]
    fn test_bfv_public_key_to_greco() {
        let (threshold_params, _) = build_pair_for_preset(BfvPreset::InsecureThreshold512).unwrap();
        let sample = UserDataEncryptionSample::generate(BfvPreset::InsecureThreshold512);

        let witness = Witness::compute(
            BfvPreset::InsecureThreshold512,
            &UserDataEncryptionCircuitInput {
                public_key: sample.public_key.clone(),
                plaintext: sample.plaintext,
            },
        )
        .unwrap();

        // Convert using our function
        let (actual_pk0is, actual_pk1is) =
            bfv_public_key_to_greco(&threshold_params, &sample.public_key).unwrap();

        // Verify the structure matches
        assert_eq!(actual_pk0is, witness.pk0is);
        assert_eq!(actual_pk1is, witness.pk1is);
    }

    #[test]
    fn test_bfv_ciphertext_to_greco() {
        let (threshold_params, _) = build_pair_for_preset(BfvPreset::InsecureThreshold512).unwrap();

        let sample = UserDataEncryptionSample::generate(BfvPreset::InsecureThreshold512);

        let witness = Witness::compute(
            BfvPreset::InsecureThreshold512,
            &UserDataEncryptionCircuitInput {
                public_key: sample.public_key.clone(),
                plaintext: sample.plaintext,
            },
        )
        .unwrap();

        let ciphertext = Ciphertext::from_bytes(&witness.ciphertext, &threshold_params).unwrap();

        // Convert using our function
        let (actual_ct0is, actual_ct1is) =
            bfv_ciphertext_to_greco(&threshold_params, &ciphertext).unwrap();

        // Verify the structure matches
        assert_eq!(actual_ct0is, witness.ct0is);
        assert_eq!(actual_ct1is, witness.ct1is);
    }
}
