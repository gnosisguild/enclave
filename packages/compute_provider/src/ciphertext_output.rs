use crate::compute_input::ComputeInput;

pub trait ComputeProvider {
    type Output: Send + Sync;

    fn prove(
        &self,
        input: &ComputeInput
    ) -> Self::Output;
}


#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ComputeResult {
    pub ciphertext_hash: Vec<u8>,
    pub params_hash: Vec<u8>,
    pub merkle_root: Vec<u8>,
}