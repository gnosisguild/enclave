use std::{cmp::Ordering, hash::Hash, mem, sync::Arc};

use actix::{Actor, Context, Handler, Message};
use anyhow::*;
use fhe::{
    bfv::{BfvParameters, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
};
use fhe_traits::Serialize;
use rand_chacha::ChaCha20Rng;

use crate::ordered_set::OrderedSet;

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash)]
#[rtype(result = "Result<(WrappedSecretKey, WrappedPublicKeyShare)>")]
pub struct GenerateKeyshare {
    // responder_pk: Vec<u8>, // TODO: use this to encrypt the secret data
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(WrappedPublicKey)>")]
pub struct GetAggregatePublicKey {
    pub keyshares: OrderedSet<WrappedPublicKeyShare>,
}

/// Wrapped PublicKeyShare. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedPublicKeyShare(pub PublicKeyShare);

impl Ord for WrappedPublicKeyShare {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bytes().cmp(&other.0.to_bytes())
    }
}

impl PartialOrd for WrappedPublicKeyShare {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<WrappedPublicKeyShare> for Vec<u8> {
    fn from(share: WrappedPublicKeyShare) -> Self {
        share.0.to_bytes()
    }
}

impl Hash for WrappedPublicKeyShare {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bytes().hash(state)
    }
}

impl WrappedPublicKeyShare {
    fn clone_inner(&self) -> PublicKeyShare {
        self.0.clone()
    } 
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedPublicKey(pub PublicKey);

impl Hash for WrappedPublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bytes().hash(state)
    }
}

impl Ord for WrappedPublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_bytes().cmp(&other.0.to_bytes())
    }
}

impl PartialOrd for WrappedPublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


#[derive(PartialEq)]
pub struct WrappedSecretKey(pub SecretKey);
impl WrappedSecretKey {
    pub fn unsafe_to_vec(&self) -> Vec<u8> {
        serialize_box_i64(self.0.coeffs.clone())
    }
}

pub struct Fhe {
    params: Arc<BfvParameters>,
    crp: CommonRandomPoly,
    rng: ChaCha20Rng,
}

impl Actor for Fhe {
    type Context = Context<Self>;
}

impl Fhe {
    pub fn new(
        params: Arc<BfvParameters>,
        crp: CommonRandomPoly,
        rng: ChaCha20Rng,
    ) -> Result<Self> {
        Ok(Self { params, crp, rng })
    }
}

impl Handler<GenerateKeyshare> for Fhe {
    type Result = Result<(WrappedSecretKey, WrappedPublicKeyShare)>;
    fn handle(&mut self, _event: GenerateKeyshare, _: &mut Self::Context) -> Self::Result {
        let sk_share = { SecretKey::random(&self.params, &mut self.rng) };
        let pk_share = { PublicKeyShare::new(&sk_share, self.crp.clone(), &mut self.rng)? };
        Ok((WrappedSecretKey(sk_share), WrappedPublicKeyShare(pk_share)))
    }
}

impl Handler<GetAggregatePublicKey> for Fhe {
    type Result = Result<WrappedPublicKey>;

    fn handle(&mut self, msg: GetAggregatePublicKey, _: &mut Self::Context) -> Self::Result {
        // Could implement Aggregate for Wrapped keys but that leaks traits
        let public_key: PublicKey = msg.keyshares.iter().map(|k| k.clone_inner()).aggregate()?;
        Ok(WrappedPublicKey(public_key))
    }
}

fn serialize_box_i64(boxed: Box<[i64]>) -> Vec<u8> {
    let vec = boxed.into_vec();
    let mut bytes = Vec::with_capacity(vec.len() * mem::size_of::<i64>());
    for &num in &vec {
        bytes.extend_from_slice(&num.to_le_bytes());
    }
    bytes
}
