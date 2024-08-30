use fhe::bfv::Plaintext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaintextSerializer {
    pub inner: Plaintext,
}

impl PlaintextSerializer {
    pub fn to_bytes(inner: Plaintext /* params: Arc<BfvParameters> */) -> Self {
        Self { inner }  
    }
}
