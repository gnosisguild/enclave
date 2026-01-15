// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::{flatten, Polynomial};
use safe::SafeSponge;

/// Generates a commitment to a set of polynomials by hashing the polynomial coefficients.
/// This is done by flattening the polynomial coefficients and hashing them.
/// TODO: Replace with a function from Enclave circuits (PVSS circuits).
///
/// # Arguments
/// * `poly0is` - The first set of polynomials.
/// * `poly1is` - The second set of polynomials.
///
/// # Returns
/// A field element representing the commitment.
pub fn generate_commitment<let N: u32, let L: u32, let BIT: u32>(
    poly0is: [Polynomial<N>; L],
    poly1is: [Polynomial<N>; L],
) -> Field {
    let mut inputs = Vec::new();

    // Flatten polynomials
    inputs = flatten::<_, _, BIT>(inputs, poly0is);
    inputs = flatten::<_, _, BIT>(inputs, poly1is);

    // Domain separator for commitment - "Greco" in hex
    let domain_separator = [
        0x47, 0x72, 0x65, 0x63, 0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    let hash = generate_hash::<L>(inputs, domain_separator, 1);

    hash.get(0)
}

/// Generates a hash of a set of field elements using a cryptographic sponge
///
/// The sponge absorbs all field elements and squeezes out a single deterministic field element
/// that will be used as the hash.
///
/// # Arguments
/// * `inputs` - The input field elements to hash.
/// * `domain_separator` - The domain separator to use for the sponge.
/// * `squeeze_count` - The number of field elements to squeeze out of the sponge.
///
/// # Returns
/// A field element representing the hash.
pub fn generate_hash<let L: u32>(
    inputs: Vec<Field>,
    domain_separator: [u8; 64],
    squeeze_count: u32,
) -> Vec<Field> {
    // IO Pattern: ABSORB(input_size), SQUEEZE(squeeze_count)
    let input_size = inputs.len();
    let io_pattern = [0x80000000 | input_size, 0x00000000 | squeeze_count];

    let mut sponge = SafeSponge::start(io_pattern, domain_separator);

    sponge.absorb(inputs);

    let hash = sponge.squeeze();

    sponge.finish();

    hash
}
