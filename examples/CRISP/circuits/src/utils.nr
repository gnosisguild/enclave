// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use lib::math::polynomial::Polynomial;

use crate::constants::HALF_LARGEST_MINIMUM_DEGREE;

/// Validates vote coefficients and ensures the vote amount doesn't exceed balance.
///
/// This function checks that:
/// 1. All vote coefficients are binary (either 0 or `q_mod_t`)
/// 2. At most one vote type is non-zero (YES and NO votes are mutually exclusive; both may be zero for no vote)
/// 3. The decoded vote amount (integer value reconstructed from binary coefficients) doesn't exceed the voter's balance
///
/// # Arguments
/// * `k1` - The plaintext polynomial encoding the vote
/// * `q_mod_t` - The field representation of 1 (Q_MOD_T_MOD_P for binary encoding)
/// * `balance` - The voter's balance/voting power
pub fn check_coefficient_values_with_balance<let D: u32>(
    k1: Polynomial<D>,
    q_mod_t: Field,
    balance: Field,
) {
    let HALF_D = D / 2;

    // Define the boundaries for YES and NO vote regions in the polynomial
    // YES votes are stored in the second half: [HALF_D, HALF_D + HALF_LARGEST_MINIMUM_DEGREE)
    let END_INDEX_Y = HALF_D + HALF_LARGEST_MINIMUM_DEGREE;
    // NO votes are stored in the first half: [0, HALF_LARGEST_MINIMUM_DEGREE)
    let END_INDEX_N = HALF_LARGEST_MINIMUM_DEGREE;

    let mut sum_yes: u64 = 0;
    let mut sum_no: u64 = 0;

    // YES votes are encoded in reverse order (LSB at highest index, MSB at lowest index)
    // We read from END_INDEX_Y-1 down to HALF_D to reconstruct the binary value
    for i in 0..HALF_LARGEST_MINIMUM_DEGREE {
        // Access coefficients in reverse order: from (END_INDEX_Y - 1) down to HALF_D
        let idx = END_INDEX_Y - 1 - i;
        let coeff = k1.coefficients[idx];

        assert(0 == coeff * (q_mod_t - coeff));

        // Convert coefficient to bit: 0 -> 0, q_mod_t -> 1
        let bit: u64 = if coeff == 0 { 0 } else { 1 };

        // Reconstruct binary value: shift left and add new bit
        // Reading backwards means we're building: bit_0 * 2^0 + bit_1 * 2^1 + ...
        sum_yes = sum_yes * 2 + bit;
    }

    // NO votes are encoded in reverse order: [END_INDEX_N-1, ..., 0]
    // We read from END_INDEX_N-1 down to 0 to reconstruct the binary value
    for i in 0..HALF_LARGEST_MINIMUM_DEGREE {
        // Access coefficients in reverse order: from (END_INDEX_N - 1) down to 0
        let idx = END_INDEX_N - 1 - i;
        let coeff = k1.coefficients[idx];

        // Binary constraint check (same as YES votes)
        assert(0 == coeff * (q_mod_t - coeff));

        // Convert coefficient to bit
        let bit: u64 = if coeff == 0 { 0 } else { 1 };

        // Reconstruct binary value
        sum_no = sum_no * 2 + bit;
    }

    if (sum_yes != 0) {
        // If YES vote is cast, NO vote must be zero
        assert(sum_no == 0);
        // Vote amount must not exceed balance
        assert(sum_yes <= (balance as u64));
    }

    if (sum_no != 0) {
        // If NO vote is cast, YES vote must be zero
        assert(sum_yes == 0);
        // Vote amount must not exceed balance
        assert(sum_no <= (balance as u64));
    }
}

/// Checks if all vote coefficients are zero.
///
/// This function verifies that both the YES and NO vote regions contain only zeros.
/// Used in masking operations to ensure that invalid or unauthorized votes are properly
/// masked (set to zero) before being added to the tally.
///
/// # Arguments
/// * `k1` - The plaintext polynomial encoding the vote
///
/// # Vote Regions Checked
/// - NO votes: coefficients [0, HALF_LARGEST_MINIMUM_DEGREE)
/// - YES votes: coefficients [HALF_D, HALF_D + HALF_LARGEST_MINIMUM_DEGREE)
pub fn check_coefficient_zero<let D: u32>(k1: Polynomial<D>) {
    let HALF_D = D / 2;

    // Define vote region boundaries
    let START_INDEX_Y = HALF_D;
    let END_INDEX_Y = HALF_D + HALF_LARGEST_MINIMUM_DEGREE;

    let START_INDEX_N = 0;
    let END_INDEX_N = HALF_LARGEST_MINIMUM_DEGREE;

    // Check YES vote region: all coefficients must be zero
    for i in START_INDEX_Y..END_INDEX_Y {
        assert(k1.coefficients[i] == 0);
    }

    // Check NO vote region: all coefficients must be zero
    for i in START_INDEX_N..END_INDEX_N {
        assert(k1.coefficients[i] == 0);
    }
}

#[test]
fn test_check_vote_is_within_balance_pass() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            // YES votes: bits at indices 50-52 (START of YES window)
            1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 1000);
}

#[test(should_fail)]
fn test_check_vote_is_within_balance_yes_and_no_not_zero() {
    let pol = Polynomial {
        coefficients: [
            // NO votes: index 0
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            // YES votes: indices 50-52
            1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 100);
}

#[test(should_fail)]
fn test_check_vote_is_within_balance_fail() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            // YES votes: indices 50-52 (binary 111 = 7, exceeds balance of 1)
            1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 1);
}

#[test]
fn test_check_coefficient_values_pass() {
    let pol = Polynomial {
        coefficients: [
            // NO vote: index 0 (binary 1 = 1)
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 100000000000);
}

#[test(should_fail)]
fn test_check_coefficient_values_fail() {
    let pol = Polynomial {
        coefficients: [
            // Invalid coefficient value 2 (not 0 or 1)
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 100);
}

#[test(should_fail)]
fn test_check_coefficient_zero_fail() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            // Non-zero at index 50 (YES section)
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_zero(pol);
}

#[test]
fn test_check_coefficient_zero() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_zero(pol);
}
