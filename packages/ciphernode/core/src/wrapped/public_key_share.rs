use std::{cmp::Ordering, hash::Hash, sync::Arc};
use fhe_traits::{Deserialize, Serialize};
use fhe::{
    bfv::BfvParameters,
    mbfv::{CommonRandomPoly, PublicKeyShare},
};
use serde::Serializer;

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
