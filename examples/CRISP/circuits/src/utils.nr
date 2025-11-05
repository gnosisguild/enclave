// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::Polynomial;

// Check that all valid coefficients are either 0 or 1 and that the vote is <= balance
pub fn check_coefficient_values_with_balance<let D: u32>(
    k1: Polynomial<D>,
    q_mod_t: Field,
    balance: Field,
) {
    // This value would allow to fit
    // 268435456 for yes and 268435456 for no
    // which would fit the supply of most tokens really (imagining one user just holds all tokens)
    let HALF_LARGEST_MINIMUM_DEGREE = 28;
    let HALF_D = D / 2;
    let START_INDEX_Y = HALF_D - HALF_LARGEST_MINIMUM_DEGREE;
    let START_INDEX_N = D - HALF_LARGEST_MINIMUM_DEGREE;

    let mut sum_yes: u64 = 0;
    let mut sum_no: u64 = 0;

    // Loop through all coefficients in the space where we could have a vote
    // yes part
    for i in START_INDEX_Y..HALF_D {
        let coeff = k1.coefficients[i];
        assert(0 == coeff * (q_mod_t - coeff));

        // evaluate using Horner's method
        sum_yes = sum_yes + sum_yes + coeff as u64;
    }

    // no part
    for i in START_INDEX_N..D {
        let coeff = k1.coefficients[i];
        assert(0 == coeff * (q_mod_t - coeff));

        // evaluate using Horner's method
        sum_no = sum_no + sum_no + coeff as u64;
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

// Check that the polynomial has 0 for all relevant coefficients
pub fn check_coefficient_zero<let D: u32>(k1: Polynomial<D>) -> bool {
    // This value would allow to fit
    // 268435456 for yes and 268435456 for no
    // which would fit the supply of most tokens really (imagining one user just holds all tokens)
    let HALF_LARGEST_MINIMUM_DEGREE = 28;
    let HALF_D = D / 2;
    let START_INDEX_Y = HALF_D - HALF_LARGEST_MINIMUM_DEGREE;
    let START_INDEX_N = D - HALF_LARGEST_MINIMUM_DEGREE;

    let mut res = true;

    // Loop through all coefficients in the space where we could have a vote
    // yes part
    for i in START_INDEX_Y..HALF_D {
        if k1.coefficients[i] != 0 {
            res = false;
        }
    }

    // no part
    for i in START_INDEX_N..D {
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
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 1000);
}

#[test(should_fail)]
fn test_check_vote_is_within_balance_yes_and_no_not_zero() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 100);
}

#[test(should_fail)]
fn test_check_vote_is_within_balance_fail() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 1);
}

#[test]
fn test_check_coefficient_values_pass() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    };

    check_coefficient_values_with_balance(pol, 1, 100000000000);
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

    check_coefficient_values_with_balance(pol, 1, 100);
}

#[test]
fn test_check_coefficient_zero_fail() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
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
