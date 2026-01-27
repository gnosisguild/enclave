//! Polynomial packing utilities for zero-knowledge circuits
//!
//! This module provides functions to pack polynomial coefficients into field elements
//! using a nibble-aligned layout, matching the Noir implementation exactly.

use ark_bn254::Fr as Field;
use ark_ff::PrimeField;
use num_bigint::BigInt;
use num_traits::Zero;

/// Compute hex-aligned packing parameters for a given `BIT`.
/// Matches the Noir `packing_layout` function exactly.
///
/// # Arguments
/// * `bit` - The bit width for coefficient bounds
///
/// # Returns
/// A tuple of (nibble_bits, group) where:
/// - `nibble_bits`: The bit width rounded up to the next multiple of 4
/// - `group`: Maximum number of limbs that fit in one BN254 field element
fn packing_layout(bit: u32) -> (u32, u32) {
    // Ceil BIT up to the next multiple of 4 (nibble alignment).
    let nibble_bits = bit.div_ceil(4) * 4;

    // Each stored limb uses an extra nibble because negative coefficients
    // will be shifted to positive, so radix = 2^(nibble_bits+4).
    assert!(nibble_bits + 4 <= 254);

    // Maximum limbs that fit in one BN254 element without wrap.
    let group = 254 / (nibble_bits + 4);
    assert!(group >= 1);
    (nibble_bits, group)
}

/// Pack values into a Vec<Field> of carriers using the shared hex-aligned layout.
///
/// Matches the Noir `packer` function exactly.
/// Packs multiple coefficients into each field element using nibble-aligned layout.
///
/// # Arguments
/// * `values` - Slice of BigInt coefficients to pack
/// * `bit` - The bit width for coefficient bounds
///
/// # Returns
/// A vector of field elements containing the packed coefficients.
/// The number of field elements is `ceil(values.len() / group)` where `group` is
/// determined by the packing layout.
fn packer(values: &[BigInt], bit: u32) -> Vec<Field> {
    // Layout parameters: nibble-aligned width and limbs-per-carrier group size.
    let (nibble_bits, group) = packing_layout(bit);

    let base = BigInt::from(2).pow(nibble_bits);
    let radix = BigInt::from(2).pow(nibble_bits + 4);

    // Number of chunks to emit: ceil(A / group).
    let a = values.len() as u32;
    let num_chunks = a.div_ceil(group);
    let mut out = Vec::new();

    // Process in fixed-size chunks of `group` limbs.
    for chunk in 0..num_chunks {
        // How many real values go into this chunk.
        let remain = a - (chunk * group);
        let take = if remain < group { remain } else { group };

        // Build field element accumulator (big-endian concatenation in `radix`).
        let mut acc = BigInt::zero();
        for i in 0..take {
            let v = &values[(chunk * group + i) as usize];
            acc = acc * &radix + (v + &base);
        }

        // Pad remaining limb slots with the canonical zero-limb `digit = base`.
        for _ in 0..(group - take) {
            acc = acc * &radix + &base;
        }

        // Convert BigInt to Field element
        let acc_biguint = if acc < BigInt::zero() {
            // Should not happen with our packing scheme, but handle it
            panic!("Negative accumulator in packer");
        } else {
            acc.to_biguint().unwrap()
        };

        // Convert to Field via bytes
        let bytes = acc_biguint.to_bytes_le();
        let field_elem = Field::from_le_bytes_mod_order(&bytes);
        out.push(field_elem);
    }
    out
}

/// Flatten `L` polynomials into a single linear stream of packed `Field` carriers.
///
/// Matches the Noir `flatten` function exactly.
/// Packs each polynomial using the same bit width and appends them sequentially.
///
/// # Arguments
/// * `inputs` - Initial vector of field elements to append to
/// * `polys` - Slice of polynomials (each represented as Vec<BigInt>)
/// * `bit` - The bit width for coefficient bounds
///
/// # Returns
/// Extended vector with packed polynomial coefficients appended in order.
/// The polynomials are packed sequentially, maintaining a stable transcript layout.
pub fn flatten(mut inputs: Vec<Field>, polys: &[Vec<BigInt>], bit: u32) -> Vec<Field> {
    for poly in polys {
        // Pack coefficients into carriers using the same BIT layout.
        let packed = packer(poly, bit);

        // Append carriers in-order to `inputs` to keep a stable transcript layout.
        inputs.extend(packed);
    }

    // Return the extended input stream.
    inputs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packing_layout() {
        // Test nibble alignment
        // For bit=1 or 4: nibble_bits=4, radix uses 4+4=8 bits, so group=254/8=31
        assert_eq!(packing_layout(1), (4, 31));
        assert_eq!(packing_layout(4), (4, 31));
        // For bit=5 or 8: nibble_bits=8, radix uses 8+4=12 bits, so group=254/12=21
        assert_eq!(packing_layout(5), (8, 21));
        assert_eq!(packing_layout(8), (8, 21));
        // For bit=51: nibble_bits=52, radix uses 52+4=56 bits, so group=254/56=4
        assert_eq!(packing_layout(51), (52, 4));
    }

    #[test]
    fn test_packer_single_value() {
        let values = vec![BigInt::from(42)];
        let packed = packer(&values, 8);
        assert!(!packed.is_empty());
    }

    #[test]
    fn test_flatten_empty() {
        let inputs = Vec::new();
        let polys: Vec<Vec<BigInt>> = vec![];
        let result = flatten(inputs, &polys, 8);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_flatten_single_poly() {
        let inputs = Vec::new();
        let poly = vec![BigInt::from(1), BigInt::from(2), BigInt::from(3)];
        let polys = vec![poly];
        let result = flatten(inputs, &polys, 8);
        assert!(!result.is_empty());
    }
}
