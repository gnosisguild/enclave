// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::Polynomial;

// Check that all valid coefficients are either 0 or q_mod_t and that the vote is <= balance
pub fn check_coefficient_values_with_balance<let D: u32>(
    k1: Polynomial<D>,
    q_mod_t: Field,
    balance: Field,
) {
    // This value allows to fit 1125899906842624 for yes and 1125899906842624 for no
    let HALF_LARGEST_MINIMUM_DEGREE = 50;
    let HALF_D = D / 2;

    // After reversal, bits that were at END of first half are now at START of second half
    let END_INDEX_Y = HALF_D + HALF_LARGEST_MINIMUM_DEGREE;

    // Bits that were at END of second half are now at START of first half
    let END_INDEX_N = HALF_LARGEST_MINIMUM_DEGREE;

    let mut sum_yes: u64 = 0;
    let mut sum_no: u64 = 0;

    // Yes part - process from END to START (reverse order)
    for i in 0..HALF_LARGEST_MINIMUM_DEGREE {
        // Access indices backwards: from (END_INDEX_Y - 1) down to START_INDEX_Y
        let idx = END_INDEX_Y - 1 - i;
        let coeff = k1.coefficients[idx];
        assert(0 == coeff * (q_mod_t - coeff));

        let bit: u64 = if coeff == 0 { 0 } else { 1 };
        sum_yes = sum_yes * 2 + bit;
    }

    // No part - process from END to START (reverse order)
    for i in 0..HALF_LARGEST_MINIMUM_DEGREE {
        let idx = END_INDEX_N - 1 - i;
        let coeff = k1.coefficients[idx];
        assert(0 == coeff * (q_mod_t - coeff));

        let bit: u64 = if coeff == 0 { 0 } else { 1 };
        sum_no = sum_no * 2 + bit;
    }

    if (sum_yes != 0) {
        assert(sum_no == 0);
        assert(sum_yes <= (balance as u64));
    }

    if (sum_no != 0) {
        assert(sum_yes == 0);
        assert(sum_no <= (balance as u64));
    }
}

pub fn check_coefficient_zero<let D: u32>(k1: Polynomial<D>) -> bool {
    let HALF_LARGEST_MINIMUM_DEGREE = 50;
    let HALF_D = D / 2;

    let START_INDEX_Y = HALF_D;
    let END_INDEX_Y = HALF_D + HALF_LARGEST_MINIMUM_DEGREE;

    let START_INDEX_N = 0;
    let END_INDEX_N = HALF_LARGEST_MINIMUM_DEGREE;

    let mut res = true;

    for i in START_INDEX_Y..END_INDEX_Y {
        if k1.coefficients[i] != 0 {
            res = false;
        }
    }

    for i in START_INDEX_N..END_INDEX_N {
        if k1.coefficients[i] != 0 {
            res = false;
        }
    }

    res
}

#[test]
fn test_check_vote_is_within_balance_pass() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 
            // YES votes: bits at indices 50-52 (START of YES window)
            1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
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

#[test]
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

    assert(check_coefficient_zero(pol) == false);
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

    assert(check_coefficient_zero(pol) == true);
}
