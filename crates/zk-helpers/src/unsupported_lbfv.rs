// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Stand-ins for l-BFV key types that this fhe.rs branch does not provide.
//!
//! Every operation fails. Callers keep their previous types so the surrounding
//! proof code still typechecks.

use fhe::aggregate::Aggregate;
use fhe::bfv::{BfvParameters, Ciphertext, SecretKey};
use fhe::Error;
use fhe_math::rq::{Ntt, NttShoup, Poly};
use rand::{CryptoRng, RngCore};
use std::sync::Arc;
use zeroize::Zeroizing;

fn unavailable() -> Error {
    Error::DefaultError("l-BFV is not available on this fhe.rs branch".to_string())
}

/// l-BFV public-key share. This branch cannot build one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKeyShare;

/// l-BFV relinearization-key share. This branch cannot build one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelinKeyShare;

/// Private rows that would prove one relinearization-key share.
#[derive(Clone)]
pub struct RlkWitness {
    pub r: Zeroizing<SecretKey>,
    pub errors_d0: Vec<Poly<NttShoup>>,
    pub errors_d2: Vec<Poly<NttShoup>>,
}

/// Aggregated l-BFV public key.
///
/// This branch has no constructor that succeeds.
#[derive(Clone, Debug)]
pub struct LBFVPublicKey {
    _private: (),
}

/// Aggregated l-BFV relinearization key.
///
/// This branch has no constructor that succeeds.
#[derive(Clone, Debug)]
pub struct LBFVRelinearizationKey {
    _private: (),
}

impl PublicKeyShare {
    pub fn from_bytes(_bytes: &[u8], _params: &Arc<BfvParameters>) -> Result<Self, Error> {
        Err(unavailable())
    }

    pub fn contribute_with_crp<R: RngCore + CryptoRng>(
        _secret_key: &SecretKey,
        _crp: &impl Sized,
        _rng: &mut R,
    ) -> Result<Self, Error> {
        Err(unavailable())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        Vec::new()
    }
}

impl RelinKeyShare {
    pub fn from_bytes(_bytes: &[u8], _params: &Arc<BfvParameters>) -> Result<Self, Error> {
        Err(unavailable())
    }

    pub fn contribution_with_crp<R: RngCore + CryptoRng>(
        _secret_key: &SecretKey,
        _urs: &impl Sized,
        _crs: &impl Sized,
        _ciphertext_level: usize,
        _key_level: usize,
        _rng: &mut R,
    ) -> Result<Self, Error> {
        Err(unavailable())
    }

    pub fn contribution_with_crp_extended<R: RngCore + CryptoRng>(
        secret_key: &SecretKey,
        _urs: &impl Sized,
        _crs: &impl Sized,
        _ciphertext_level: usize,
        _key_level: usize,
        _rng: &mut R,
    ) -> Result<(Self, RlkWitness), Error> {
        let _ = secret_key;
        Err(unavailable())
    }

    pub fn ciphertext_level(&self) -> usize {
        0
    }

    pub fn key_level(&self) -> usize {
        0
    }

    pub fn d0_components(&self) -> &[Poly<Ntt>] {
        &[]
    }

    pub fn d2_components(&self) -> &[Poly<Ntt>] {
        &[]
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        Vec::new()
    }
}

impl PublicKeyShare {
    pub fn a_components(&self) -> Result<Vec<Poly<Ntt>>, Error> {
        Err(unavailable())
    }

    pub fn b_components(&self) -> Result<Vec<Poly<Ntt>>, Error> {
        Err(unavailable())
    }
}

impl LBFVPublicKey {
    pub fn from_bytes(_bytes: &[u8], _params: &Arc<BfvParameters>) -> Result<Self, Error> {
        Err(unavailable())
    }

    pub fn parameters(&self) -> Arc<BfvParameters> {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn seed(&self) -> Option<[u8; 32]> {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn row_count(&self) -> usize {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn rows(&self) -> &[Ciphertext] {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn try_encrypt<R: RngCore + CryptoRng>(
        &self,
        _plaintext: &fhe::bfv::Plaintext,
        _rng: &mut R,
    ) -> Result<Ciphertext, Error> {
        Err(unavailable())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }
}

impl LBFVRelinearizationKey {
    pub fn from_bytes(_bytes: &[u8], _params: &Arc<BfvParameters>) -> Result<Self, Error> {
        Err(unavailable())
    }

    pub fn reconstruct_public_key(&self) -> Result<LBFVPublicKey, Error> {
        Err(unavailable())
    }

    pub fn parameters(&self) -> Arc<BfvParameters> {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn ciphertext_level(&self) -> usize {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn key_level(&self) -> usize {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn decomposition_log_base(&self) -> usize {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn d0_components(&self) -> &[Poly<NttShoup>] {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn d1_components(&self) -> &[Poly<NttShoup>] {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn d2_components(&self) -> &[Poly<NttShoup>] {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn a_components(&self) -> &[Poly<NttShoup>] {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn b_components(&self) -> &[Poly<NttShoup>] {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }

    pub fn relinearizes(&self, _ciphertext: &mut Ciphertext) -> Result<(), Error> {
        Err(unavailable())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        unimplemented!("l-BFV is not available on this fhe.rs branch")
    }
}

impl Aggregate<PublicKeyShare> for LBFVPublicKey {
    fn from_shares<T>(_iter: T) -> Result<Self, Error>
    where
        T: IntoIterator<Item = PublicKeyShare>,
    {
        Err(unavailable())
    }
}

pub fn aggregate_relinearization_key(
    _shares: &[RelinKeyShare],
    _public_key: &LBFVPublicKey,
) -> Result<LBFVRelinearizationKey, Error> {
    Err(unavailable())
}
