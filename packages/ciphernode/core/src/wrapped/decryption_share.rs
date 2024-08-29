use anyhow::*;
use fhe::{
    bfv::{BfvParameters, Ciphertext},
    mbfv::DecryptionShare,
};
use fhe_traits::{Deserialize, DeserializeParametrized, Serialize};
use std::sync::Arc;

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
