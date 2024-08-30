use fhe::bfv::Plaintext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrappedPlaintext {
    pub inner: Plaintext,
}

impl WrappedPlaintext {
    pub fn from_fhe_rs(inner: Plaintext /* params: Arc<BfvParameters> */) -> Self {
        Self { inner }  
    }
}
