// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// This is a placeholder.
pub struct Polynomial<let N: u32> {
    pub coefficients: [Field; N],
}

impl<let N: u32> Polynomial<N> {
    pub fn new(coefficients: [Field; N]) -> Self {
        Polynomial { coefficients }
    }
}
