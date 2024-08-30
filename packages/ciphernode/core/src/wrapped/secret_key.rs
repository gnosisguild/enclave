use anyhow::*;
use fhe::bfv::{BfvParameters, SecretKey};
use fhe_traits::{Deserialize, Serialize};
use std::sync::Arc;

/// Wrapped SecretKey. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
// We should favor consuming patterns and avoid cloning and copying this value around in memory.
// Underlying key Zeroizes on drop
pub struct SecretKeySerializer {
    pub inner: SecretKey,
    pub params: Arc<BfvParameters>,
}

impl SecretKeySerializer {
    pub fn to_bytes(inner: SecretKey, params: Arc<BfvParameters>) -> Result<Vec<u8>> {
        let value = Self { inner, params };
        Ok(value.unsafe_serialize()?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<SecretKey> {
        Ok(Self::deserialize(bytes)?.inner)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SecretKeyData {
    coeffs: Box<[i64]>,
    par: Vec<u8>,
}

impl SecretKeySerializer {
    pub fn unsafe_serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&SecretKeyData {
            coeffs: self.inner.coeffs.clone(),
            par: self.params.clone().to_bytes(),
        })?)
    }

    pub fn deserialize(bytes: &[u8]) -> Result<SecretKeySerializer> {
        let SecretKeyData { coeffs, par } = bincode::deserialize(&bytes)?;
        let params = Arc::new(BfvParameters::try_deserialize(&par).unwrap());
        Ok(Self {
            inner: SecretKey::new(coeffs.to_vec(), &params),
            params,
        })
    }
}
