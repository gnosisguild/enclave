// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::Polynomial;

// Check that a vote is not greater than the available balance
// This assumes that the vote is encoded inside the polynomial
// coefficients and that they are checked to be either 0 or 1
pub fn check_vote_is_within_balance<let D: u32>(k1: Polynomial<D>, balance: Field) {
    // This value would allow to fit
    // 268435456 for yes and 268435456 for no
    // which would fit the supply of most tokens really (imagining one user just holds all tokens)
    let HALF_LARGEST_MINIMUM_DEGREE: u32 = 28;
    let HALF_D = D / 2;
    let START_INDEX_Y = HALF_D - HALF_LARGEST_MINIMUM_DEGREE;
    let START_INDEX_N = D - HALF_LARGEST_MINIMUM_DEGREE;

    // we assume that a check that the coefficients are either 0 or 1 was done already
    let mut sum_yes: u64 = 0;
    let mut sum_no: u64 = 0;

    // yes part
    for i in START_INDEX_Y..HALF_D {
        let coeff = k1.coefficients[i] as u64;
        let position = i - START_INDEX_Y;
        let exponent = HALF_LARGEST_MINIMUM_DEGREE - 1 - position;
        let weight: u64 = (1 << exponent) as u64;
        sum_yes += coeff * weight;
    }

    // no part
    for i in START_INDEX_N..D {
        let coeff = k1.coefficients[i] as u64;
        let position = i - START_INDEX_N;
        let exponent = HALF_LARGEST_MINIMUM_DEGREE - 1 - position;
        let weight: u64 = (1 << exponent) as u64;
        sum_no += coeff * weight;
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

// Check that all valid coefficients are either 0 or 1
pub fn check_coefficient_values<let D: u32>(k1: Polynomial<D>, q_mod_t: Field) {
    // This value would allow to fit
    // 268435456 for yes and 268435456 for no
    // which would fit the supply of most tokens really (imagining one user just holds all tokens)
    let HALF_LARGEST_MINIMUM_DEGREE = 28;
    let HALF_D = D / 2;
    let START_INDEX_Y = HALF_D - HALF_LARGEST_MINIMUM_DEGREE;
    let START_INDEX_N = D - HALF_LARGEST_MINIMUM_DEGREE;

    // Loop through all coefficients in the space where we could have a vote
    // yes part
    for i in START_INDEX_Y..HALF_D {
        let coeff = k1.coefficients[i];
        assert(0 == coeff * (q_mod_t - coeff));
    }

    // no part
    for i in START_INDEX_N..D {
        let coeff = k1.coefficients[i];
        assert(0 == coeff * (q_mod_t - coeff));
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

    check_vote_is_within_balance(pol, 1000);
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

    check_vote_is_within_balance(pol, 100);
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

    check_vote_is_within_balance(pol, 6);
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

    check_coefficient_values(pol, 1);
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

    check_coefficient_values(pol, 1);
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
