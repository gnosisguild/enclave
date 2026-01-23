// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// Half of the largest minimum degree used for vote encoding regions.
///
/// This constant defines the size of the coefficient region allocated for encoding vote amounts
/// in binary format within each half of the polynomial. Votes are encoded as binary numbers:
/// - NO votes: coefficients [0, HALF_LARGEST_MINIMUM_DEGREE)
/// - YES votes: coefficients [HALF_D, HALF_D + HALF_LARGEST_MINIMUM_DEGREE)
///
/// With a value of 50, each vote type can represent values up to 2^50 - 1.
pub global HALF_LARGEST_MINIMUM_DEGREE: u32 = 50;
