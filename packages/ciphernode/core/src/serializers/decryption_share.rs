use anyhow::*;
use fhe::{
    bfv::{BfvParameters, Ciphertext},
    mbfv::DecryptionShare,
};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use serde::Serializer;
use std::sync::Arc;

pub struct DecryptionShareSerializer {
    inner: DecryptionShare,
    params: Arc<BfvParameters>,
    ct: Arc<Ciphertext>,
}

impl DecryptionShareSerializer {
    pub fn to_bytes(
        inner: DecryptionShare,
        params: Arc<BfvParameters>,
        ct: Arc<Ciphertext>,
    ) -> Result<Vec<u8>> {
        let value = Self { inner, params, ct };
        Ok(bincode::serialize(&value)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<DecryptionShare> {
        let ds: DecryptionShareSerializer = bincode::deserialize(&bytes)?;
        Ok(ds.inner)
    }
}

/// Deserialize from serde to PublicKeySerializer
impl<'de> serde::Deserialize<'de> for DecryptionShareSerializer {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Intermediate struct of bytes for deserialization
        #[derive(serde::Deserialize)]
        struct DecryptionShareBytes {
            par: Vec<u8>,
            bytes: Vec<u8>,
            ct: Vec<u8>,
        }
        let DecryptionShareBytes { par, bytes, ct } =
            DecryptionShareBytes::deserialize(deserializer)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par).unwrap()); // TODO: fix errors
        let ct = Arc::new(Ciphertext::from_bytes(&ct, &params).unwrap()); // TODO: fix errors
        let inner = DecryptionShare::deserialize(&bytes, &params, ct.clone())
            .map_err(serde::de::Error::custom)?;
        // TODO: how do we create an invariant that the deserialized params match the global params?
        std::result::Result::Ok(Self { inner, params, ct })
    }
}
/// Serialize to serde bytes representation
impl serde::Serialize for DecryptionShareSerializer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let bytes = self.inner.to_bytes();
        let par_bytes = self.params.to_bytes();
        let ct_bytes = self.ct.to_bytes();
        // Intermediate struct of bytes
        let mut state = serializer.serialize_struct("DecryptionShareBytes", 2)?;
        state.serialize_field("par", &par_bytes)?;
        state.serialize_field("bytes", &bytes)?;
        state.serialize_field("bytes", &ct_bytes)?;
        state.end()
    }
}
