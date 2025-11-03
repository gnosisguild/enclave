// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use polynomial::Polynomial;

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

    let mut sum = 0;

    // Loop through all coefficients in the space where we could have a vote
    // yes part
    for i in START_INDEX_Y..HALF_D {
        sum += k1.coefficients[i];
    }

    // no part
    for i in START_INDEX_N..D {
        sum += k1.coefficients[i];
    }

   sum == 0
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

#[test(should_fail)]
fn test_check_coefficient_zero_fail() {
    let pol = Polynomial {
        coefficients: [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
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
