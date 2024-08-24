use std::{cmp::Ordering, hash::Hash, mem, sync::Arc};

use actix::{Actor, Context, Handler, Message};
use anyhow::*;
use fhe::{
    bfv::{BfvParameters, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, PublicKeyShare},
};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use rand_chacha::ChaCha20Rng;
use serde::Serializer;
// use serde::{Deserialize, Serialize};

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
pub struct WrappedPublicKeyShare {
    inner: PublicKeyShare,
    params: Arc<BfvParameters>,
    crp: CommonRandomPoly,
}

impl WrappedPublicKeyShare {
    pub fn from_fhe_rs(
        inner: PublicKeyShare,
        params: Arc<BfvParameters>,
        crp: CommonRandomPoly,
    ) -> Self {
        Self { inner, params, crp }
    }

    pub fn clone_inner(&self) -> PublicKeyShare {
        self.inner.clone()
    }
}

impl Ord for WrappedPublicKeyShare {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.to_bytes().cmp(&other.inner.to_bytes())
    }
}

impl PartialOrd for WrappedPublicKeyShare {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<WrappedPublicKeyShare> for Vec<u8> {
    fn from(share: WrappedPublicKeyShare) -> Self {
        share.inner.to_bytes()
    }
}

impl Hash for WrappedPublicKeyShare {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.to_bytes().hash(state)
    }
}

/// Deserialize from serde to WrappedPublicKeyShare
impl<'de> serde::Deserialize<'de> for WrappedPublicKeyShare {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Intermediate struct for deserialization
        #[derive(serde::Deserialize)]
        struct PublicKeyShareBytes {
            par_bytes: Vec<u8>,
            crp_bytes: Vec<u8>,
            bytes: Vec<u8>,
        }
        let PublicKeyShareBytes {
            par_bytes,
            crp_bytes,
            bytes,
        } = PublicKeyShareBytes::deserialize(deserializer)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par_bytes).unwrap());
        let crp =
            CommonRandomPoly::deserialize(&crp_bytes, &params).map_err(serde::de::Error::custom)?;
        let inner = PublicKeyShare::deserialize(&bytes, &params, crp.clone())
            .map_err(serde::de::Error::custom)?;
        std::result::Result::Ok(WrappedPublicKeyShare::from_fhe_rs(inner, params, crp))
    }
}

/// Serialize to intermediate struct
impl serde::Serialize for WrappedPublicKeyShare {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        // let par = self.0.
        let bytes = self.inner.to_bytes();
        let par_bytes = self.params.to_bytes();
        let crp_bytes = self.params.to_bytes();
        let mut state = serializer.serialize_struct("PublicKeyShare", 2)?;
        state.serialize_field("par_bytes", &par_bytes)?;
        state.serialize_field("crp_bytes", &crp_bytes)?;
        state.serialize_field("bytes", &bytes)?;
        state.end()
    }
}

/// Wrapped PublicKey. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedPublicKey {
    inner: PublicKey,
    params: Arc<BfvParameters>,
}

impl WrappedPublicKey {
    pub fn from_fhe_rs(inner: PublicKey, params: Arc<BfvParameters>) -> Self {
        Self { inner, params }
    }
}

impl fhe_traits::Serialize for WrappedPublicKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.inner.to_bytes()
    }
}

/// Deserialize from serde to WrappedPublicKey
impl<'de> serde::Deserialize<'de> for WrappedPublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct PublicKeyBytes {
            par: Vec<u8>,
            bytes: Vec<u8>,
        }
        let PublicKeyBytes { par, bytes } = PublicKeyBytes::deserialize(deserializer)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par).unwrap());
        let inner = PublicKey::from_bytes(&bytes, &params).map_err(serde::de::Error::custom)?;
        std::result::Result::Ok(WrappedPublicKey::from_fhe_rs(inner, params))
    }
}

/// Serialize to intermediate struct
impl serde::Serialize for WrappedPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        // let par = self.0.
        let bytes = self.inner.to_bytes();
        let par_bytes = self.params.to_bytes();
        let mut state = serializer.serialize_struct("PublicKey", 2)?;
        state.serialize_field("par_bytes", &par_bytes)?;
        state.serialize_field("bytes", &bytes)?;
        state.end()
    }
}

impl Hash for WrappedPublicKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.to_bytes().hash(state)
    }
}

impl Ord for WrappedPublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.to_bytes().cmp(&other.inner.to_bytes())
    }
}

impl PartialOrd for WrappedPublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Wrapped SecretKey. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
// We should favor consuming patterns and avoid cloning and copying this value around in memory.
// Underlying key Zeroizes on drop
#[derive(PartialEq)]
pub struct WrappedSecretKey(pub SecretKey);

impl WrappedSecretKey {
    pub fn unsafe_to_vec(&self) -> Vec<u8> {
        serialize_box_i64(self.0.coeffs.clone())
    }
}

/// Fhe library adaptor. All FHE computations should happen through this actor.
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
        Ok((
            WrappedSecretKey(sk_share),
            WrappedPublicKeyShare::from_fhe_rs(pk_share, self.params.clone(), self.crp.clone()),
        ))
    }
}

impl Handler<GetAggregatePublicKey> for Fhe {
    type Result = Result<WrappedPublicKey>;

    fn handle(&mut self, msg: GetAggregatePublicKey, _: &mut Self::Context) -> Self::Result {
        // Could implement Aggregate for Wrapped keys but that leaks traits
        let public_key: PublicKey = msg.keyshares.iter().map(|k| k.clone_inner()).aggregate()?;
        Ok(WrappedPublicKey::from_fhe_rs(
            public_key,
            self.params.clone(),
        ))
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
