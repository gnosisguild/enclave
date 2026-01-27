// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Utility functions for zkFHE circuit generation
//!
//! This module contains helper functions for:
//! - String conversion of BigInt vectors
//! - SAFE sponge hash computation
//! - BigInt to Field element conversion
//! - Bit width calculation from bounds
//! - ZKP modulus constants

use ark_bn254::Fr as Field;
use ark_bn254::Fr as FieldElement;
use ark_ff::PrimeField;
use e3_safe::SafeSponge;
use num_bigint::BigInt;
use num_traits::Zero;
use std::str::FromStr;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum ZkHelpersUtilsError {
    #[error("Failed to parse bound: {0}")]
    ParseBound(String),
}

pub type Result<T> = std::result::Result<T, ZkHelpersUtilsError>;

/// Convert a 1D vector of BigInt to a vector of strings.
///
/// # Arguments
/// * `vec` - Slice of BigInt values
///
/// # Returns
/// A vector of strings, one per BigInt value
pub fn to_string_1d_vec(vec: &[BigInt]) -> Vec<String> {
    vec.iter().map(|x| x.to_string()).collect()
}

/// Convert a 2D vector of BigInt to a vector of vectors of strings.
///
/// # Arguments
/// * `poly` - Slice of BigInt vectors (e.g., polynomial coefficients per modulus)
///
/// # Returns
/// A 2D vector of strings
pub fn to_string_2d_vec(poly: &[Vec<BigInt>]) -> Vec<Vec<String>> {
    poly.iter().map(|row| to_string_1d_vec(row)).collect()
}

/// Convert a 3D vector of BigInt to a vector of vectors of vectors of strings.
///
/// # Arguments
/// * `vec` - 3D slice of BigInt values
///
/// # Returns
/// A 3D vector of strings
pub fn to_string_3d_vec(vec: &[Vec<Vec<BigInt>>]) -> Vec<Vec<Vec<String>>> {
    vec.iter().map(|d1| to_string_2d_vec(d1)).collect()
}

/// Compute SAFE sponge hash with the given domain separator and inputs.
///
/// This is a convenience wrapper around the SAFE sponge API that performs
/// START, ABSORB, SQUEEZE, and FINISH operations in sequence.
///
/// # Arguments
/// * `domain_separator` - 64-byte domain separator for cross-protocol security
/// * `inputs` - Vector of field elements to absorb
/// * `io_pattern` - IO pattern array `[ABSORB(input_size), SQUEEZE(output_size)]`
///
/// # Returns
/// A vector of field elements squeezed from the sponge
pub fn compute_safe(
    domain_separator: [u8; 64],
    inputs: Vec<Field>,
    io_pattern: [u32; 2],
) -> Vec<Field> {
    let mut sponge = SafeSponge::start(io_pattern, domain_separator);
    sponge.absorb(inputs);
    let digests = sponge.squeeze();
    sponge.finish();

    digests
}

/// Convert BigInt to Field by reducing modulo ZKP modulus.
///
/// This is a helper to simplify BigInt to Field conversion.
/// Handles negative values by reducing them to the positive range [0, ZKP_MODULUS).
///
/// # Arguments
/// * `value` - BigInt value to convert
///
/// # Returns
/// A field element representing the value modulo ZKP modulus
pub fn bigint_to_field(value: &BigInt) -> FieldElement {
    let zkp_modulus = get_zkp_modulus();
    let reduced = if value < &BigInt::zero() {
        (value % &zkp_modulus) + &zkp_modulus
    } else {
        value % &zkp_modulus
    };
    let biguint = reduced
        .to_biguint()
        .unwrap_or_else(|| (&zkp_modulus + reduced).to_biguint().unwrap());
    let bytes = biguint.to_bytes_le();
    FieldElement::from_le_bytes_mod_order(&bytes)
}

/// Calculate bit width from a bound string.
///
/// The formula is: BIT = ceil(log₂(bound)) + 1
///
/// # Arguments
/// * `bound_str` - String representation of the bound value
///
/// # Returns
/// The calculated bit width, or an error if the bound cannot be parsed
///
/// # Errors
/// Returns `ZkHelpersUtilsError::ParseBound` if the bound string cannot be parsed as a BigInt
pub fn calculate_bit_width(bound_str: &str) -> Result<u32> {
    let bound = BigInt::from_str(bound_str)
        .map_err(|e| ZkHelpersUtilsError::ParseBound(format!("{bound_str}: {e}")))?;

    if bound <= BigInt::from(0) {
        return Ok(1); // Minimum 1 bit
    }

    // Calculate log2 and add 1
    let log2 = bound.bits() as f64;
    let bit_width = (log2.ceil() as u32) + 1;

    Ok(bit_width)
}

/// Get the ZKP modulus as a BigInt.
///
/// The ZKP modulus is the BN254 scalar field modulus:
/// 21888242871839275222246405745257275088548364400416034343698204186575808495617
///
/// # Returns
/// The ZKP modulus as a BigInt
///
/// # Panics
/// Panics if the modulus constant is invalid (should never happen)
pub fn get_zkp_modulus() -> BigInt {
    BigInt::from_str(
        "21888242871839275222246405745257275088548364400416034343698204186575808495617",
    )
    .expect("Invalid ZKP modulus")
}
