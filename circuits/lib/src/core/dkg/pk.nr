// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::math::commitments::compute_dkg_pk_commitment;
use crate::math::polynomial::Polynomial;

/// Correct DKG Public Key Circuit (Circuit 0).
pub struct Pk<let N: u32, let L: u32, let BIT_PK: u32> {
    /// Correct DKG public key components
    /// pk0[i] is the first component for modulus i
    pk0: [Polynomial<N>; L],
    /// pk1[i] is the second component for modulus i
    pk1: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32> Pk<N, L, BIT_PK> {
    pub fn new(pk0: [Polynomial<N>; L], pk1: [Polynomial<N>; L]) -> Self {
        Pk { pk0, pk1 }
    }

    /// Main verification function
    /// Returns commitment to correct DKG public key
    pub fn execute(self) -> Field {
        compute_dkg_pk_commitment::<N, L, BIT_PK>(self.pk0, self.pk1)
    }
}
