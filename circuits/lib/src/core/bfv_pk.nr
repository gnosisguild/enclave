use crate::math::commitments::{compute_pk_bfv_commitment, prepare_pk_commitment_payload};
use crate::math::polynomial::Polynomial;

/// BFV Public Key Commitment Circuit (Circuit 0).
///
/// commit to the BFV public key for later verification.
/// No validation of pk correctness - that's caught by decryption failures in Circuit 4.
pub struct BfvPkCommit<let N: u32, let L: u32, let BIT_PK: u32> {
    /// BFV public key components (public input)
    /// pk0[i] is the first component for modulus i
    pk0: [Polynomial<N>; L],
    /// pk1[i] is the second component for modulus i
    pk1: [Polynomial<N>; L],
}

impl<let N: u32, let L: u32, let BIT_PK: u32> BfvPkCommit<N, L, BIT_PK> {
    pub fn new(pk0: [Polynomial<N>; L], pk1: [Polynomial<N>; L]) -> Self {
        BfvPkCommit { pk0, pk1 }
    }

    /// Main verification function
    /// Returns commitment to BFV public key
    pub fn verify(self) -> Field {
        // Compute and return the pk bfv commitment
        let payload = prepare_pk_commitment_payload::<N, L, BIT_PK>(self.pk0, self.pk1);
        compute_pk_bfv_commitment(payload)
    }
}
