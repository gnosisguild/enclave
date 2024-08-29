use anyhow::*;
use fhe::bfv::{BfvParameters, SecretKey};
use fhe_traits::{Deserialize, Serialize};
use std::sync::Arc;

/// Wrapped SecretKey. This is wrapped to provide an inflection point
/// as we use this library elsewhere we only implement traits as we need them
/// and avoid exposing underlying structures from fhe.rs
// We should favor consuming patterns and avoid cloning and copying this value around in memory.
// Underlying key Zeroizes on drop
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedSecretKey {
    pub inner: SecretKey,
    pub params: Arc<BfvParameters>,
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
