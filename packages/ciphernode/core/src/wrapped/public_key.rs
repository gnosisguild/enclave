use anyhow::*;
use fhe::bfv::{BfvParameters, PublicKey};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use serde::Serializer;
use std::sync::Arc;

/// Wrapped PublicKey. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
pub struct PublicKeySerializer {
    inner: PublicKey,
    params: Arc<BfvParameters>,
}

impl PublicKeySerializer {
    pub fn to_bytes(inner: PublicKey, params: Arc<BfvParameters>) -> Result<Vec<u8>> {
        let value = Self { inner, params };
        Ok(bincode::serialize(&value)?)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<PublicKey> {
        let wpk: PublicKeySerializer = bincode::deserialize(&bytes)?;
        Ok(wpk.inner)
    }
}

/// Deserialize from serde to PublicKeySerializer
impl<'de> serde::Deserialize<'de> for PublicKeySerializer {
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
        std::result::Result::Ok(Self { inner, params })
    }
}

/// Serialize to serde bytes representation
impl serde::Serialize for PublicKeySerializer {
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

// impl Hash for PublicKeySerializer {
//     fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
//         self.inner.to_bytes().hash(state)
//     }
// }
//
// impl Ord for PublicKeySerializer {
//     fn cmp(&self, other: &Self) -> Ordering {
//         self.inner.to_bytes().cmp(&other.inner.to_bytes())
//     }
// }
//
// impl PartialOrd for PublicKeySerializer {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }
