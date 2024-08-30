use anyhow::*;
use fhe::{
    bfv::{BfvParameters, Ciphertext},
    mbfv::DecryptionShare,
};
use fhe_traits::Serialize;
use std::sync::Arc;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DecryptionShareSerializer {
    inner: Vec<u8>,
    params: Vec<u8>,
    ct: Vec<u8>,
}

impl DecryptionShareSerializer {
    pub fn to_bytes(
        inner: DecryptionShare,
        params: Arc<BfvParameters>,
        ct: Arc<Ciphertext>,
    ) -> Result<Vec<u8>> {
        // Have to serialize immediately in order to clone etc.
        let inner_bytes = inner.to_bytes();
        let params_bytes = params.to_bytes();
        let ct_bytes = ct.to_bytes();
        let value = Self {
            inner: inner_bytes,
            params: params_bytes,
            ct: ct_bytes,
        };
        Ok(bincode::serialize(&value)?)
    }
}
