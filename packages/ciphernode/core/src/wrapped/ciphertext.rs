use fhe::bfv::{BfvParameters, Ciphertext};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use serde::Serializer;
use std::{hash::Hash, sync::Arc};

/// Wrapped Ciphertext. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedCiphertext {
    pub inner: Ciphertext,
    pub params: Arc<BfvParameters>,
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
