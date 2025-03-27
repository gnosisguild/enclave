#[allow(dead_code)]
use fhe::bfv::{BfvParametersBuilder, BfvParameters};
use std::{
    sync::Arc,
    error::Error,
};

/// Generate BFV parameters
/// 
/// # Returns
/// 
/// * A BFV parameters
pub fn generate_bfv_parameters(
) -> Result<Arc<BfvParameters>, Box<dyn Error + Send + Sync>> {
    let degree = 2048;
    let plaintext_modulus: u64 = 1032193;
    let moduli = vec![0xffffffff00001];

    Ok(BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(&moduli)
        .build_arc()?)
}