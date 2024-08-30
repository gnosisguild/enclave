use anyhow::*;
use fhe::bfv::{BfvParameters, Ciphertext};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use serde::Serializer;
use std::sync::Arc;

/// Wrapped Ciphertext. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
pub struct CiphertextSerializer {
    pub inner: Ciphertext,
    pub params: Arc<BfvParameters>,
}

impl CiphertextSerializer {
    pub fn to_bytes(inner: Ciphertext, params: Arc<BfvParameters>) -> Result<Vec<u8>> {
        let value = Self { inner, params };
        Ok(bincode::serialize(&value)?)
    }

    pub fn from_bytes(bytes:&[u8]) -> Result<Ciphertext>{
        let wct: Self = bincode::deserialize(&bytes)?;
        Ok(wct.inner)
    }
}

/// Deserialize from serde to PublicKeySerializer
impl<'de> serde::Deserialize<'de> for CiphertextSerializer {
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
        std::result::Result::Ok(Self { inner, params })
    }
}
impl serde::Serialize for CiphertextSerializer {
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
