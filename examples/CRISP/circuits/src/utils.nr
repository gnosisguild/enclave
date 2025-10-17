// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::Polynomial;

// Calculate the indices at which to check whether the encoded binary value is
// either 0 or 1
pub fn calculate_indices(degree: u64, total_voting_power: Field) -> [u64; 2] {
    let half_degree = degree / 2;

    // Convert to big-endian bits (most significant bit first)
    let bits: [u1; 254] = total_voting_power.to_be_bits();

    // Find the position of the first '1' bit (most significant bit)
    let mut bits_needed: u64 = 0;
    let mut found_first_bit = false;

    for i in 0..254 {
        if (bits[i] == 1) & !found_first_bit {
            bits_needed = 254 - (i as u64);
            found_first_bit = true;
        }
    }

    // Handle edge case
    if bits_needed == 0 {
        bits_needed = 1;
    }

    let yes_index = half_degree - bits_needed;
    let no_index = degree - bits_needed;

    [yes_index, no_index]
}

#[test]
fn test_calculate_indices() {
    let res = calculate_indices(8192, 100);

    // bitsNeeded = 7 -> 1100100 = 100 in binary
    // half length = 4096
    // first valid index for yes 4096 - 7 = 4089
    // first valid index for no 8192 - 7 = 8185

    assert(res[0] == 4089);
    assert(res[1] == 8185);
}

// Check that all valid coefficients are either 0 or 1
// @todo change to generic once we upgrade the version of Greco - this is for testing only
pub fn check_coefficient_values(
    k1: Polynomial<100>,
    degree: u64,
    total_voting_power: Field,
    q_mod_t: Field,
) {
    let valid_indices = calculate_indices(degree, total_voting_power);
    let half_degree = degree / 2;

    // Loop through all coefficients but only check those in valid ranges
    for i in 0..100 {
        let i_u64 = i as u64;

        // Check if we're in the valid range for yes votes (first half)
        if (i_u64 >= valid_indices[0]) & (i_u64 < half_degree) {
            let coeff = k1.coefficients[i];
            assert(0 == coeff * (q_mod_t - coeff));
        }

        // Check if we're in the valid range for no votes (second half)
        if (i_u64 >= valid_indices[1]) & (i_u64 < degree) {
            let coeff = k1.coefficients[i];
            assert(0 == coeff * (q_mod_t - coeff));
        }
    }
}

#[test]
fn test_check_coefficient_values_pass() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values(pol, 100, 5, 1);
}

#[test(should_fail)]
fn test_check_coefficient_values_fail() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ],
    };

    check_coefficient_values(pol, 100, 5, 1);
}
