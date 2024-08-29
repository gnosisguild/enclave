use fhe::bfv::{BfvParameters, PublicKey};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use serde::Serializer;
use std::{cmp::Ordering, hash::Hash, sync::Arc};

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
