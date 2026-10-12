// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Coefficient representations for polynomial computations.

use ndarray::Array2;
use num_bigint::BigInt;

/// Copy an Array2 of unsigned coefficients into BigInt coefficients.
pub fn array2_u64_to_bigint(arr: &Array2<u64>) -> Array2<BigInt> {
    let (rows, cols) = arr.dim();
    let mut out = Array2::<BigInt>::zeros((rows, cols));
    for i in 0..rows {
        for j in 0..cols {
            out[[i, j]] = BigInt::from(arr[[i, j]]);
        }
    }
    out
}
