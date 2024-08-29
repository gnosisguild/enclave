use std::{cmp::Ordering, hash::Hash, mem, sync::Arc};

use crate::ordered_set::OrderedSet;
use actix::{Actor, Context, Handler, Message};
use anyhow::*;
use fhe::{
    bfv::{BfvParameters, BfvParametersBuilder, Ciphertext, Plaintext, PublicKey, SecretKey},
    mbfv::{AggregateIter, CommonRandomPoly, DecryptionShare, PublicKeyShare},
};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::Serializer;

// TODO: remove all this wrapping and serialization/deserialization code by ensuring everything from fhe.rs has a to_bytes() and deserialize() -> T methods and return only Vec<u8> outside of this actor

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

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(WrappedPlaintext)>")]
pub struct GetAggregatePlaintext {
    pub decryptions: OrderedSet<WrappedDecryptionShare>,
}

#[derive(Message, Clone, Debug, PartialEq, Eq)]
#[rtype(result = "Result<(WrappedDecryptionShare)>")]
pub struct DecryptCiphertext {
    pub unsafe_secret: WrappedSecretKey,
    pub ciphertext: WrappedCiphertext,
}

/// Wrapped PublicKeyShare. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedPublicKeyShare {
    inner: PublicKeyShare,
    // We need to hold copies of the params and crp in order to effectively serialize and
    // deserialize the wrapped type
    params: Arc<BfvParameters>,
    crp: CommonRandomPoly,
}

impl WrappedPublicKeyShare {
    /// Public function to serialize specifically from the wrapped type including types that are
    /// private from outside the crate
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
        // Intermediate struct of bytes for deserialization
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
        // TODO: how do we create an invariant that the deserialized params match the global params?
        std::result::Result::Ok(WrappedPublicKeyShare::from_fhe_rs(inner, params, crp))
    }
}

/// Serialize to serde bytes representation
impl serde::Serialize for WrappedPublicKeyShare {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let bytes = self.inner.to_bytes();
        let par_bytes = self.params.to_bytes();
        let crp_bytes = self.crp.to_bytes();
        // Intermediate struct of bytes
        let mut state = serializer.serialize_struct("PublicKeyShare", 3)?;
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
        // Intermediate struct of bytes for deserialization
        #[derive(serde::Deserialize)]
        struct PublicKeyBytes {
            par: Vec<u8>,
            bytes: Vec<u8>,
        }
        let PublicKeyBytes { par, bytes } = PublicKeyBytes::deserialize(deserializer)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par).unwrap()); // TODO: fix errors
        let inner = PublicKey::from_bytes(&bytes, &params).map_err(serde::de::Error::custom)?;
        // TODO: how do we create an invariant that the deserialized params match the global params?
        std::result::Result::Ok(WrappedPublicKey::from_fhe_rs(inner, params))
    }
}

/// Serialize to serde bytes representation
impl serde::Serialize for WrappedPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let bytes = self.inner.to_bytes();
        let par_bytes = self.params.to_bytes();
        // Intermediate struct of bytes
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedSecretKey {
    inner: SecretKey,
    params: Arc<BfvParameters>,
}

impl WrappedSecretKey {
    pub fn from_fhe_rs(inner: SecretKey, params: Arc<BfvParameters>) -> Self {
        Self { inner, params }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SecretKeyData {
    coeffs: Box<[i64]>,
    par: Vec<u8>,
}

impl WrappedSecretKey {
    pub fn unsafe_serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&SecretKeyData {
            coeffs: self.inner.coeffs.clone(),
            par: self.params.clone().to_bytes(),
        })?)
    }

    pub fn deserialize(bytes: Vec<u8>) -> Result<WrappedSecretKey> {
        let SecretKeyData { coeffs, par } = bincode::deserialize(&bytes)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par).unwrap());
        Ok(WrappedSecretKey::from_fhe_rs(
            SecretKey::new(coeffs.to_vec(), &params),
            params,
        ))
    }
}

/// Wrapped Ciphertext. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedCiphertext {
    inner: Ciphertext,
    params: Arc<BfvParameters>,
}

impl WrappedCiphertext {
    pub fn from_fhe_rs(inner: Ciphertext, params: Arc<BfvParameters>) -> Self {
        Self { inner, params }
    }
}

impl Hash for WrappedCiphertext {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.to_bytes().hash(state)
    }
}

/// Deserialize from serde to WrappedPublicKey
impl<'de> serde::Deserialize<'de> for WrappedCiphertext {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Intermediate struct of bytes for deserialization
        #[derive(serde::Deserialize)]
        struct DeserializedBytes {
            par: Vec<u8>,
            bytes: Vec<u8>,
        }
        let DeserializedBytes { par, bytes } = DeserializedBytes::deserialize(deserializer)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par).unwrap());
        let inner = Ciphertext::from_bytes(&bytes, &params).map_err(serde::de::Error::custom)?;
        std::result::Result::Ok(WrappedCiphertext::from_fhe_rs(inner, params))
    }
}
impl serde::Serialize for WrappedCiphertext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let bytes = self.inner.to_bytes();
        let par_bytes = self.params.to_bytes();
        // Intermediate struct of bytes
        let mut state = serializer.serialize_struct("Ciphertext", 2)?;
        state.serialize_field("par_bytes", &par_bytes)?;
        state.serialize_field("bytes", &bytes)?;
        state.end()
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct WrappedDecryptionShare {
    inner: Vec<u8>,
    params: Vec<u8>,
    ct: Vec<u8>,
}

impl WrappedDecryptionShare {
    pub fn from_fhe_rs(
        inner: DecryptionShare,
        params: Arc<BfvParameters>,
        ct: Arc<Ciphertext>,
    ) -> Self {
        // Have to serialize immediately in order to clone etc.
        let inner_bytes = inner.to_bytes();
        let params_bytes = params.to_bytes();
        let ct_bytes = ct.to_bytes();
        Self {
            inner: inner_bytes,
            params: params_bytes,
            ct: ct_bytes,
        }
    }

    pub fn try_inner(self) -> Result<DecryptionShare> {
        let params = Arc::new(BfvParameters::try_deserialize(&self.params)?);
        let ct = Arc::new(Ciphertext::from_bytes(&self.ct, &params)?);
        Ok(DecryptionShare::deserialize(&self.inner, &params, ct)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedPlaintext {
    inner: Plaintext,
}

impl WrappedPlaintext {
    pub fn from_fhe_rs(inner: Plaintext /* params: Arc<BfvParameters> */) -> Self {
        Self { inner }
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
    pub fn try_default() -> Result<Self> {
        let moduli = &vec![0x3FFFFFFF000001];
        let degree = 2048usize;
        let plaintext_modulus = 1032193u64;
        let mut rng = ChaCha20Rng::from_entropy();
        let params = BfvParametersBuilder::new()
            .set_degree(degree)
            .set_plaintext_modulus(plaintext_modulus)
            .set_moduli(&moduli)
            .build_arc()?;
        let crp = CommonRandomPoly::new(&params, &mut rng)?;

        Ok(Fhe::new(params, crp, rng)?)
    }
}

impl Handler<GenerateKeyshare> for Fhe {
    type Result = Result<(WrappedSecretKey, WrappedPublicKeyShare)>;
    fn handle(&mut self, _event: GenerateKeyshare, _: &mut Self::Context) -> Self::Result {
        let sk_share = { SecretKey::random(&self.params, &mut self.rng) };
        let pk_share = { PublicKeyShare::new(&sk_share, self.crp.clone(), &mut self.rng)? };
        Ok((
            WrappedSecretKey::from_fhe_rs(sk_share, self.params.clone()),
            WrappedPublicKeyShare::from_fhe_rs(pk_share, self.params.clone(), self.crp.clone()),
        ))
    }
}

impl Handler<DecryptCiphertext> for Fhe {
    type Result = Result<WrappedDecryptionShare>;
    fn handle(&mut self, msg: DecryptCiphertext, _: &mut Self::Context) -> Self::Result {
        let DecryptCiphertext {
            unsafe_secret, // TODO: fix security issues with sending secrets between actors
            ciphertext,
        } = msg;

        let ct = Arc::new(ciphertext.inner);
        let inner = DecryptionShare::new(&unsafe_secret.inner, &ct, &mut self.rng).unwrap();

        Ok(WrappedDecryptionShare::from_fhe_rs(
            inner,
            ciphertext.params,
            ct.clone(),
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

impl Handler<GetAggregatePlaintext> for Fhe {
    type Result = Result<WrappedPlaintext>;
    fn handle(&mut self, msg: GetAggregatePlaintext, _: &mut Self::Context) -> Self::Result {
        let plaintext: Plaintext = msg
            .decryptions
            .iter()
            .map(|k| k.clone().try_inner())
            .collect::<Result<Vec<_>>>()? // NOTE: not optimal
            .into_iter()
            .aggregate()?;

        Ok(WrappedPlaintext::from_fhe_rs(plaintext))
    }
}
