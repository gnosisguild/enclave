//! Commitment computation functions for zero-knowledge circuits.
//!
//! This module provides functions to compute commitments to various cryptographic objects
//! (polynomials, public keys, secret keys, shares, etc.) using the SAFE sponge hash function.
//! All functions match the corresponding Noir circuit implementations exactly.

use crate::packing::flatten;
use crate::utils::compute_safe;
use ark_bn254::Fr as Field;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use num_bigint::BigInt;

// ============================================================================
// DOMAIN SEPARATORS
// ============================================================================

/// Domain separator for BFV public key commitments.
/// String: "PK_BFV"
const DS_PK_BFV: [u8; 64] = [
    0x50, 0x4b, 0x5f, 0x42, 0x46, 0x56, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for TRBFV public key commitments.
/// String: "PK_TRBFV"
const DS_PK_TRBFV: [u8; 64] = [
    0x50, 0x4b, 0x5f, 0x54, 0x52, 0x42, 0x46, 0x56, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for secret commitments (sk_trbfv or e_sm).
/// String: "SECRET"
const DS_SECRET: [u8; 64] = [
    0x53, 0x45, 0x43, 0x52, 0x45, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for shares party-modulus commitments.
/// String: "SPM"
const DS_SPM: [u8; 64] = [
    0x53, 0x50, 0x4d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for aggregated shares commitments.
/// String: "AGG_SHARES"
const DS_AGG_SHARES: [u8; 64] = [
    0x41, 0x47, 0x47, 0x5f, 0x53, 0x48, 0x41, 0x52, 0x45, 0x53, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for public key aggregation commitments.
/// String: "PK_AGG"
const DS_PK_AGG: [u8; 64] = [
    0x50, 0x4b, 0x5f, 0x41, 0x47, 0x47, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for decryption share challenge commitments.
/// String: "AGGREGATION"
const DS_AGGREGATION: [u8; 64] = [
    0x41, 0x47, 0x47, 0x72, 0x65, 0x67, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for public key TRBFV challenge.
/// String: "CLG_PK_TRBFV"
const DS_CLG_PK_TRBFV: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x50, 0x4b, 0x5f, 0x54, 0x52, 0x42, 0x46, 0x56, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for encryption BFV challenge.
/// String: "CLG_ENC_BFV"
const DS_CLG_ENC_BFV: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x45, 0x4e, 0x43, 0x5f, 0x42, 0x46, 0x56, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for Greco challenge.
/// String: "CLG_GRECO"
const DS_CLG_GRECO: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x47, 0x72, 0x65, 0x63, 0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for decryption share challenge.
/// String: "CLG_DEC_SHARE"
const DS_CLG_DEC_SHARE: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x44, 0x65, 0x63, 0x53, 0x68, 0x61, 0x72, 0x65, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for general-purpose ciphertext commitments.
/// String: "CIPHERTEXT"
const DS_CIPHERTEXT: [u8; 64] = [
    0x43, 0x49, 0x50, 0x48, 0x45, 0x52, 0x54, 0x45, 0x58, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// ============================================================================
// WRAPPERS
// ============================================================================

/// Compute commitments using SAFE sponge with the given domain separator and payload.
///
/// This matches the Noir `compute_commitments` function exactly.
///
/// # Arguments
/// * `payload` - Vector of field elements to hash
/// * `domain_separator` - Domain separator for cross-protocol security
/// * `io_pattern` - IO pattern `[ABSORB(input_size), SQUEEZE(output_size)]`
///
/// # Returns
/// A vector of field elements from the sponge
pub fn compute_commitments(
    payload: Vec<Field>,
    domain_separator: [u8; 64],
    io_pattern: [u32; 2],
) -> Vec<Field> {
    compute_safe(domain_separator, payload, io_pattern)
}

/// Compute a commitment to the BFV public key polynomials by flattening them and hashing.
///
/// This matches the Noir `compute_pk_bfv_commitment` function exactly.
///
/// # Arguments
/// * `pk0` - First component of the BFV public key (one vector per modulus)
/// * `pk1` - Second component of the BFV public key (one vector per modulus)
/// * `bit_pk` - The bit width for public key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_pk_bfv_commitment(pk0: &[Vec<BigInt>], pk1: &[Vec<BigInt>], bit_pk: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, pk0, bit_pk);
    payload = flatten(payload, pk1, bit_pk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_PK_BFV, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute a commitment to the TRBFV public key polynomials by flattening them and hashing.
///
/// This matches the Noir `compute_pk_trbfv_commitment` function exactly.
///
/// # Arguments
/// * `pk0` - First component of the TRBFV public key (one vector per modulus)
/// * `pk1` - Second component of the TRBFV public key (one vector per modulus)
/// * `bit_pk` - The bit width for public key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_pk_trbfv_commitment(
    pk0: &[Vec<BigInt>],
    pk1: &[Vec<BigInt>],
    bit_pk: u32,
) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, pk0, bit_pk);
    payload = flatten(payload, pk1, bit_pk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_PK_TRBFV, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute a commitment to the secret key polynomial by flattening it and hashing.
///
/// This matches the Noir `compute_secret_sk_commitment` function exactly.
///
/// # Arguments
/// * `sk` - Secret key polynomial coefficients
/// * `bit_sk` - The bit width for secret key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_secret_sk_commitment(sk: &[BigInt], bit_sk: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &[sk.to_vec()], bit_sk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SECRET, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute a commitment to the smudging noise (e_sm).
///
/// This matches the Noir `compute_secret_e_sm_commitment` function exactly.
///
/// # Arguments
/// * `e_sm` - Smudging noise polynomial coefficients (one vector per modulus)
/// * `bit_e_sm` - The bit width for smudging noise coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_secret_e_sm_commitment(e_sm: &[Vec<BigInt>], bit_e_sm: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, e_sm, bit_e_sm);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SECRET, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

// ============================================================================
// COMMITMENTS
// ============================================================================

/// Compute SPM commitment from message polynomial.
///
/// This matches the Noir `compute_spm_commitment_from_message` function exactly.
///
/// # Arguments
/// * `message` - Message polynomial coefficients
/// * `bit_msg` - The bit width for message coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_spm_commitment_from_message(message: &[BigInt], bit_msg: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &[message.to_vec()], bit_msg);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SPM, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute SPM commitment from shares.
///
/// This matches the Noir `compute_spm_commitment_from_shares` function exactly.
/// Used in C2 (verify shares circuit).
///
/// # Arguments
/// * `y` - 3D array of share values: `y[coeff_idx][mod_idx][party_idx]`
/// * `party_idx` - Index of the party (0-based)
/// * `mod_idx` - Index of the modulus
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_spm_commitment_from_shares(
    y: &[Vec<Vec<BigInt>>],
    party_idx: usize,
    mod_idx: usize,
) -> BigInt {
    let mut payload = Vec::new();

    // Add shares y[coeff_idx][mod_idx][party_idx + 1] for each coefficient
    for coeff_y in y {
        let share_value = &coeff_y[mod_idx][party_idx + 1];
        payload.push(crate::utils::bigint_to_field(share_value));
    }

    // Include party_idx and mod_idx in the hash
    payload.push(Field::from(party_idx as u64));
    payload.push(Field::from(mod_idx as u64));

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SPM, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute public key aggregation commitment.
///
/// This matches the Noir `compute_pk_agg_commitment` function exactly.
///
/// # Arguments
/// * `pk0` - First component of the public key (one vector per modulus)
/// * `pk1` - Second component of the public key (one vector per modulus)
/// * `bit_pk` - The bit width for public key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_pk_agg_commitment(pk0: &[Vec<BigInt>], pk1: &[Vec<BigInt>], bit_pk: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, pk0, bit_pk);
    payload = flatten(payload, pk1, bit_pk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_PK_AGG, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute aggregation commitment.
///
/// This matches the Noir `compute_aggregation_commitment` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_aggregation_commitment(payload: Vec<Field>) -> BigInt {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_AGGREGATION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute CRISP ciphertext commitment.
///
/// # Arguments
/// * `ct0` - First component of the ciphertext (one vector per modulus)
/// * `ct1` - Second component of the ciphertext (one vector per modulus)
/// * `bit_ct` - The bit width for ciphertext coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_ciphertext_commitment(
    ct0: &[Vec<BigInt>],
    ct1: &[Vec<BigInt>],
    bit_ct: u32,
) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, ct0, bit_ct);
    payload = flatten(payload, ct1, bit_ct);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_CIPHERTEXT, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute aggregated shares commitment (either sk_shares or e_sm_shares).
///
/// This matches the Noir `compute_aggregated_shares_commitment` function exactly.
///
/// # Arguments
/// * `agg_shares` - Array of aggregated share polynomials (one per modulus)
/// * `bit_msg` - The bit width for message coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_aggregated_shares_commitment(agg_shares: &[Vec<BigInt>], bit_msg: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, agg_shares, bit_msg);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_AGG_SHARES, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

// ============================================================================
// COMMITMENTS FOR CHALLENGES
// ============================================================================

/// Compute public key TRBFV challenge.
///
/// This matches the Noir `compute_pk_trbfv_challenge` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
/// * `l` - Number of moduli
///
/// # Returns
/// A vector of `BigInt` challenges (2*L elements)
pub fn compute_pk_trbfv_challenge(payload: Vec<Field>, l: usize) -> Vec<BigInt> {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, (2 * l as u32)];

    compute_commitments(payload, DS_CLG_PK_TRBFV, io_pattern)
        .into_iter()
        .map(|challenge_field| {
            let challenge_bytes = challenge_field.into_bigint().to_bytes_le();
            BigInt::from_bytes_le(num_bigint::Sign::Plus, &challenge_bytes)
        })
        .collect()
}

/// Compute BFV encryption challenge.
///
/// This matches the Noir `compute_bfv_enc_challenge` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
/// * `l` - Number of moduli
///
/// # Returns
/// A vector of `BigInt` challenges (2*L elements)
pub fn compute_bfv_enc_challenge(payload: Vec<Field>, l: usize) -> Vec<BigInt> {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, (2 * l as u32)];

    compute_commitments(payload, DS_CLG_ENC_BFV, io_pattern)
        .into_iter()
        .map(|challenge_field| {
            let challenge_bytes = challenge_field.into_bigint().to_bytes_le();
            BigInt::from_bytes_le(num_bigint::Sign::Plus, &challenge_bytes)
        })
        .collect()
}

/// Compute Greco challenge commitment.
///
/// This matches the Noir `compute_greco_challenge_commitment` function exactly.
/// Verifies pk_commitment using pk0is and pk1is, then generates challenges from gammas_payload.
///
/// # Arguments
/// * `pk0is` - First component of public keys (one vector per modulus)
/// * `pk1is` - Second component of public keys (one vector per modulus)
/// * `gammas_payload` - Payload for generating challenges
/// * `pk_commitment` - Expected public key commitment value
/// * `bit_pk` - The bit width for public key coefficient bounds
/// * `l` - Number of moduli
///
/// # Returns
/// A vector of `BigInt` challenges (2*L elements)
///
/// # Panics
/// Panics if the computed public key commitment doesn't match `pk_commitment`
pub fn compute_greco_challenge_commitment(
    pk0is: &[Vec<BigInt>],
    pk1is: &[Vec<BigInt>],
    gammas_payload: Vec<Field>,
    pk_commitment: &BigInt,
    bit_pk: u32,
    l: usize,
) -> Vec<BigInt> {
    // Verify pk_commitment matches the commitment from pk0is and pk1is
    let computed_pk_commitment = compute_pk_agg_commitment(pk0is, pk1is, bit_pk);
    if computed_pk_commitment != *pk_commitment {
        panic!(
            "PK commitment mismatch in Greco circuit: expected {}, got {}",
            pk_commitment, computed_pk_commitment
        );
    }

    let input_size = gammas_payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, (2 * l as u32)];

    compute_commitments(gammas_payload, DS_CLG_GRECO, io_pattern)
        .into_iter()
        .map(|challenge_field| {
            let challenge_bytes = challenge_field.into_bigint().to_bytes_le();
            BigInt::from_bytes_le(num_bigint::Sign::Plus, &challenge_bytes)
        })
        .collect()
}

/// Compute decryption share challenge.
///
/// This matches the Noir `compute_dec_share_challenge` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_dec_share_challenge(payload: Vec<Field>) -> BigInt {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_CLG_DEC_SHARE, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}
