// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

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
use e3_polynomial::{CrtPolynomial, Polynomial};
use num_bigint::BigInt;
use std::slice::from_ref;

// ============================================================================
// DOMAIN SEPARATORS
// ============================================================================

/// String: "PK"
const DS_PK: [u8; 64] = [
    0x50, 0x4b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "PK_GENERATION"
const DS_PK_GENERATION: [u8; 64] = [
    0x50, 0x4b, 0x5f, 0x47, 0x45, 0x4e, 0x45, 0x52, 0x41, 0x54, 0x49, 0x4f, 0x4e, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "SHARE_COMPUTATION"
const DS_SHARE_COMPUTATION: [u8; 64] = [
    0x53, 0x48, 0x41, 0x52, 0x45, 0x5f, 0x43, 0x4f, 0x4d, 0x50, 0x55, 0x54, 0x41, 0x54, 0x49, 0x4f,
    0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "SHARE_ENCRYPTION"
const DS_SHARE_ENCRYPTION: [u8; 64] = [
    0x53, 0x48, 0x41, 0x52, 0x45, 0x5f, 0x45, 0x4e, 0x43, 0x52, 0x59, 0x50, 0x54, 0x49, 0x4f, 0x4e,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "PK_AGGREGATION"
const DS_PK_AGGREGATION: [u8; 64] = [
    0x50, 0x4b, 0x5f, 0x41, 0x47, 0x47, 0x52, 0x45, 0x47, 0x41, 0x54, 0x49, 0x4f, 0x4e, 0x00, 0x00,
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

/// String: "AGGREGATED_SHARES"
const DS_AGGREGATED_SHARES: [u8; 64] = [
    0x41, 0x47, 0x47, 0x52, 0x45, 0x47, 0x41, 0x54, 0x45, 0x44, 0x5f, 0x53, 0x48, 0x41, 0x52, 0x45,
    0x53, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "RECURSIVE_AGGREGATION"
const DS_RECURSIVE_AGGREGATION: [u8; 64] = [
    0x52, 0x45, 0x43, 0x55, 0x52, 0x53, 0x49, 0x56, 0x45, 0x5f, 0x41, 0x47, 0x47, 0x52, 0x45, 0x47,
    0x41, 0x54, 0x49, 0x4f, 0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "CLG_PK_GENERATION"
const DS_CLG_PK_GENERATION: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x50, 0x4b, 0x5f, 0x47, 0x45, 0x4e, 0x45, 0x52, 0x41, 0x54, 0x49, 0x4f,
    0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "CLG_SHARE_ENCRYPTION"
const DS_CLG_SHARE_ENCRYPTION: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x53, 0x48, 0x41, 0x52, 0x45, 0x5f, 0x45, 0x4e, 0x43, 0x52, 0x59, 0x50,
    0x54, 0x49, 0x4f, 0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// String: "CLG_USER_DATA_ENCRYPTION"
const DS_CLG_USER_DATA_ENCRYPTION: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x55, 0x53, 0x45, 0x52, 0x5f, 0x44, 0x41, 0x54, 0x41, 0x5f, 0x45, 0x4e,
    0x43, 0x52, 0x59, 0x50, 0x54, 0x49, 0x4f, 0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// Domain separator for decryption share challenge.
/// String: "CLG_SHARE_DECRYPTION"
const DS_CLG_SHARE_DECRYPTION: [u8; 64] = [
    0x43, 0x4c, 0x47, 0x5f, 0x53, 0x48, 0x41, 0x52, 0x45, 0x5f, 0x44, 0x45, 0x43, 0x52, 0x59, 0x50,
    0x54, 0x49, 0x4f, 0x4e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
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

// ============================================================================
// COMMITMENTS
// ============================================================================

/// Compute a commitment to the correct DKG public key polynomials by flattening them and hashing.
///
/// This matches the Noir `compute_pk_generation_commitment` function exactly.
///
/// # Arguments
/// * `pk0` - First component of the correct DKG public key (one vector per modulus)
/// * `pk1` - Second component of the correct DKG public key (one vector per modulus)
/// * `bit_pk` - The bit width for public key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_dkg_pk_commitment(pk0: &CrtPolynomial, pk1: &CrtPolynomial, bit_pk: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &pk0.limbs, bit_pk);
    payload = flatten(payload, &pk1.limbs, bit_pk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_PK, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute a commitment to the threshold public key polynomials by flattening them and hashing.
///
/// This matches the Noir `compute_threshold_pk_commitment` function exactly.
///
/// # Arguments
/// * `pk0` - First component of the thershold public key (CRT limbs)
/// * `pk1` - Second component of the thershold public key (CRT limbs)
/// * `bit_pk` - The bit width for public key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_threshold_pk_commitment(
    pk0: &CrtPolynomial,
    pk1: &CrtPolynomial,
    bit_pk: u32,
) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &pk0.limbs, bit_pk);
    payload = flatten(payload, &pk1.limbs, bit_pk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_PK_GENERATION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute a commitment to the threshold secret key share by flattening it and hashing.
///
/// This matches the Noir `compute_share_computation_sk_commitment` function exactly.
///
/// # Arguments
/// * `sk` - Threshold secret key polynomial
/// * `bit_sk` - The bit width for threshold secret key share coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_share_computation_sk_commitment(sk: &Polynomial, bit_sk: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, from_ref(sk), bit_sk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SHARE_COMPUTATION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute a commitment to the threshold smudging noise share by flattening it and hashing.
///
/// This matches the Noir `compute_share_computation_e_sm_commitment` function exactly.
///
/// # Arguments
/// * `e_sm` - Threshold smudging noise polynomial (CRT limbs)
/// * `bit_e_sm` - The bit width for threshold smudging noise share coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_share_computation_e_sm_commitment(e_sm: &CrtPolynomial, bit_e_sm: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &e_sm.limbs, bit_e_sm);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SHARE_COMPUTATION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute share encryption commitment from message polynomial.
///
/// This matches the Noir `compute_share_encryption_commitment_from_message` function exactly.
///
/// # Arguments
/// * `message` - Message polynomial
/// * `bit_msg` - The bit width for message coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_share_encryption_commitment_from_message(
    message: &Polynomial,
    bit_msg: u32,
) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, from_ref(message), bit_msg);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SHARE_ENCRYPTION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute share encryption commitment from shares.
///
/// This matches the Noir `compute_share_encryption_commitment_from_shares` function exactly.
/// Used in C2 (verify shares circuit).
///
/// # Arguments
/// * `y` - 3D array of share values: `y[coeff_idx][mod_idx][party_idx]`
/// * `party_idx` - Index of the party (0-based)
/// * `mod_idx` - Index of the modulus
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_share_encryption_commitment_from_shares(
    y: &[Vec<Vec<BigInt>>],
    party_idx: usize,
    mod_idx: usize,
) -> BigInt {
    let mut payload = Vec::new();

    // Add shares y[coeff_idx][mod_idx][party_idx + 1] for each coefficient
    for coeff_y in y {
        let share_value = coeff_y.get(mod_idx).expect("Modulus index out of bounds");
        let share_value = share_value
            .get(party_idx + 1)
            .expect("Party index out of bounds");
        payload.push(crate::utils::bigint_to_field(share_value));
    }

    // Include party_idx and mod_idx in the hash
    payload.push(Field::from(party_idx as u64));
    payload.push(Field::from(mod_idx as u64));

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_SHARE_ENCRYPTION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute threshold public key aggregation commitment.
///
/// This matches the Noir `compute_pk_aggregation_commitment` function exactly.
///
/// # Arguments
/// * `pk0` - First component of the threshold public key (CRT limbs)
/// * `pk1` - Second component of the threshold public key (CRT limbs)
/// * `bit_pk` - The bit width for threshold public key coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_pk_aggregation_commitment(
    pk0: &CrtPolynomial,
    pk1: &CrtPolynomial,
    bit_pk: u32,
) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &pk0.limbs, bit_pk);
    payload = flatten(payload, &pk1.limbs, bit_pk);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_PK_AGGREGATION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute aggregation commitment.
///
/// This matches the Noir `compute_recursive_aggregation_commitment` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_recursive_aggregation_commitment(payload: Vec<Field>) -> BigInt {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_RECURSIVE_AGGREGATION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

/// Compute CRISP ciphertext commitment.
///
/// # Arguments
/// * `ct0` - First component of the ciphertext (CRT limbs)
/// * `ct1` - Second component of the ciphertext (CRT limbs)
/// * `bit_ct` - The bit width for ciphertext coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_ciphertext_commitment(
    ct0: &CrtPolynomial,
    ct1: &CrtPolynomial,
    bit_ct: u32,
) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &ct0.limbs, bit_ct);
    payload = flatten(payload, &ct1.limbs, bit_ct);

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
/// * `agg_shares` - Aggregated share polynomial (CRT limbs)
/// * `bit_msg` - The bit width for message coefficient bounds
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_aggregated_shares_commitment(agg_shares: &CrtPolynomial, bit_msg: u32) -> BigInt {
    let mut payload = Vec::new();
    payload = flatten(payload, &agg_shares.limbs, bit_msg);

    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_AGGREGATED_SHARES, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

// ============================================================================
// COMMITMENTS FOR CHALLENGES
// ============================================================================

/// Compute public key generation challenge.
///
/// This matches the Noir `compute_threshold_pk_challenge` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
/// * `l` - Number of moduli
///
/// # Returns
/// A vector of `BigInt` challenges (2*L elements)
pub fn compute_threshold_pk_challenge(payload: Vec<Field>, l: usize) -> Vec<BigInt> {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, (2 * l as u32)];

    compute_commitments(payload, DS_CLG_PK_GENERATION, io_pattern)
        .into_iter()
        .map(|challenge_field| {
            let challenge_bytes = challenge_field.into_bigint().to_bytes_le();
            BigInt::from_bytes_le(num_bigint::Sign::Plus, &challenge_bytes)
        })
        .collect()
}

/// Compute share encryption challenge.
///
/// This matches the Noir `compute_share_encryption_challenge` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
/// * `l` - Number of moduli
///
/// # Returns
/// A vector of `BigInt` challenges (2*L elements)
pub fn compute_share_encryption_challenge(payload: Vec<Field>, l: usize) -> Vec<BigInt> {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, (2 * l as u32)];

    compute_commitments(payload, DS_CLG_SHARE_ENCRYPTION, io_pattern)
        .into_iter()
        .map(|challenge_field| {
            let challenge_bytes = challenge_field.into_bigint().to_bytes_le();
            BigInt::from_bytes_le(num_bigint::Sign::Plus, &challenge_bytes)
        })
        .collect()
}

/// Compute User Data Encryption challenge commitment.
///
/// This matches the Noir `compute_user_data_encryption_challenge_commitment` function exactly.
/// Verifies pk_commitment using pk0is and pk1is, then generates challenges from gammas_payload.
///
/// # Arguments
/// * `pk0is` - First component of public keys (CRT limbs)
/// * `pk1is` - Second component of public keys (CRT limbs)
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
pub fn compute_user_data_encryption_challenge_commitment(
    pk0is: &CrtPolynomial,
    pk1is: &CrtPolynomial,
    gammas_payload: Vec<Field>,
    pk_commitment: &BigInt,
    bit_pk: u32,
    l: usize,
) -> Vec<BigInt> {
    let computed_pk_commitment = compute_pk_aggregation_commitment(pk0is, pk1is, bit_pk);
    if computed_pk_commitment != *pk_commitment {
        panic!(
            "PK commitment mismatch in User Data Encryption circuit: expected {}, got {}",
            pk_commitment, computed_pk_commitment
        );
    }

    let input_size = gammas_payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, (2 * l as u32)];

    compute_commitments(gammas_payload, DS_CLG_USER_DATA_ENCRYPTION, io_pattern)
        .into_iter()
        .map(|challenge_field| {
            let challenge_bytes = challenge_field.into_bigint().to_bytes_le();
            BigInt::from_bytes_le(num_bigint::Sign::Plus, &challenge_bytes)
        })
        .collect()
}

/// Compute threshold share decryption challenge.
///
/// This matches the Noir `compute_threshold_share_decryption_challenge` function exactly.
///
/// # Arguments
/// * `payload` - Prepared payload as a vector of field elements
///
/// # Returns
/// A `BigInt` representing the commitment hash value
pub fn compute_threshold_share_decryption_challenge(payload: Vec<Field>) -> BigInt {
    let input_size = payload.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment_field = compute_commitments(payload, DS_CLG_SHARE_DECRYPTION, io_pattern)[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();
    BigInt::from_bytes_le(num_bigint::Sign::Plus, &commitment_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::bigint_to_field;
    use e3_polynomial::CrtPolynomial;

    fn field_to_bigint(value: Field) -> BigInt {
        let bytes = value.into_bigint().to_bytes_le();
        BigInt::from_bytes_le(num_bigint::Sign::Plus, &bytes)
    }

    #[test]
    fn compute_ciphertext_commitment_matches_manual_payload() {
        let bit_ct = 4;
        let ct0 = CrtPolynomial::from_bigint_vectors(vec![vec![BigInt::from(1), BigInt::from(2)]]);
        let ct1 = CrtPolynomial::from_bigint_vectors(vec![vec![BigInt::from(3), BigInt::from(4)]]);

        let mut payload = Vec::new();
        payload = flatten(payload, &ct0.limbs, bit_ct);
        payload = flatten(payload, &ct1.limbs, bit_ct);

        let input_size = payload.len() as u32;
        let io_pattern = [0x80000000 | input_size, 1];
        let expected = field_to_bigint(compute_commitments(payload, DS_CIPHERTEXT, io_pattern)[0]);

        let actual = compute_ciphertext_commitment(&ct0, &ct1, bit_ct);
        assert_eq!(actual, expected);
    }

    #[test]
    fn compute_share_encryption_commitment_from_shares_matches_manual_payload() {
        let y = vec![
            vec![
                vec![BigInt::from(0), BigInt::from(11), BigInt::from(12)],
                vec![BigInt::from(0), BigInt::from(21), BigInt::from(22)],
            ],
            vec![
                vec![BigInt::from(0), BigInt::from(13), BigInt::from(14)],
                vec![BigInt::from(0), BigInt::from(23), BigInt::from(24)],
            ],
            vec![
                vec![BigInt::from(0), BigInt::from(15), BigInt::from(16)],
                vec![BigInt::from(0), BigInt::from(25), BigInt::from(26)],
            ],
        ];
        let party_idx = 0;
        let mod_idx = 1;

        let mut payload = Vec::new();
        for coeff_y in &y {
            let share_value = &coeff_y[mod_idx][party_idx + 1];
            payload.push(bigint_to_field(share_value));
        }
        payload.push(Field::from(party_idx as u64));
        payload.push(Field::from(mod_idx as u64));

        let input_size = payload.len() as u32;
        let io_pattern = [0x80000000 | input_size, 1];
        let expected =
            field_to_bigint(compute_commitments(payload, DS_SHARE_ENCRYPTION, io_pattern)[0]);

        let actual = compute_share_encryption_commitment_from_shares(&y, party_idx, mod_idx);
        assert_eq!(actual, expected);
    }

    #[test]
    fn compute_threshold_pk_challenge_returns_2l_elements() {
        let payload = vec![Field::from(1u64), Field::from(2u64)];
        let l = 3;

        let challenges = compute_threshold_pk_challenge(payload, l);
        assert_eq!(challenges.len(), 2 * l);
    }

    #[test]
    fn compute_share_encryption_challenge_returns_2l_elements() {
        let payload = vec![Field::from(1u64), Field::from(2u64)];
        let l = 3;

        let challenges = compute_share_encryption_challenge(payload, l);
        assert_eq!(challenges.len(), 2 * l);
    }

    #[test]
    fn compute_recursive_aggregation_commitment_matches_manual_payload() {
        let payload = vec![Field::from(1u64), Field::from(2u64)];

        let input_size = payload.len() as u32;
        let io_pattern = [0x80000000 | input_size, 1];
        let expected = field_to_bigint(
            compute_commitments(payload.clone(), DS_RECURSIVE_AGGREGATION, io_pattern)[0],
        );

        let actual = compute_recursive_aggregation_commitment(payload);
        assert_eq!(actual, expected);
    }

    #[test]
    fn compute_threshold_share_decryption_challenge_returns_single_bigint() {
        let payload = vec![Field::from(1u64), Field::from(2u64)];

        let input_size = payload.len() as u32;
        let io_pattern = [0x80000000 | input_size, 1];
        let expected = field_to_bigint(
            compute_commitments(payload.clone(), DS_CLG_SHARE_DECRYPTION, io_pattern)[0],
        );

        let actual = compute_threshold_share_decryption_challenge(payload);
        assert_eq!(actual, expected);
    }
}
