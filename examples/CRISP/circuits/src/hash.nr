// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::{flatten, Polynomial};
use safe::SafeSponge;

/// Generates a commitment to a set of polynomials by hashing the polynomial coefficients.
/// This is done by flattening the polynomial coefficients and hashing them.
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

    let hash = generate_hash::<L>(inputs);

    hash.get(0)
}

/// Generates a hash of a set of field elements using a cryptographic sponge
///
/// The sponge absorbs all field elements and squeezes out a single deterministic field element
/// that will be used as the hash.
///
/// # Returns
/// A field element representing the hash.
pub fn generate_hash<let L: u32>(inputs: Vec<Field>) -> Vec<Field> {
    // Domain separator for CRISP - "CRISP" in hex
    let domain_separator = [
        0x43, 0x52, 0x49, 0x53, 0x50, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    // IO Pattern: ABSORB(input_size), SQUEEZE(2*L)
    let input_size = inputs.len();
    let io_pattern = [0x80000000 | input_size, 0x00000000 | (2 * L)];

    let mut sponge = SafeSponge::start(io_pattern, domain_separator);

    sponge.absorb(inputs);

    let hash = sponge.squeeze();

    sponge.finish();

    hash
}
