#[allow(dead_code)]
use fhe::bfv::{BfvParametersBuilder, BfvParameters};
use std::{
    sync::Arc,
    error::Error,
};
use alloy::primitives::{U256};
use alloy::{
    dyn_abi::{DynSolValue, DynSolType}
};

pub fn generate_bfv_parameters(
) -> Result<Arc<BfvParameters>, Box<dyn Error + Send + Sync>> {
    let degree = 2048;
    let plaintext_modulus: u64 = 1032193;
    let moduli = vec![0xffffffff00001];

    Ok(BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(&moduli)
        .build_arc()
        .unwrap()
    )
}

pub fn encode_bfv_params(moduli: Vec<u64>, degree: u64, plaintext_modulus: u64) -> Vec<u8> {
    let degree_value = U256::from(degree);
    let plaintext_value = U256::from(plaintext_modulus);
    let moduli_values = moduli.iter()
        .map(|&m| U256::from(m))
        .collect::<Vec<_>>();
    
    let params_tuple = DynSolValue::Tuple(vec![
        DynSolValue::Uint(degree_value, 256), 
        DynSolValue::Uint(plaintext_value, 256), 
        DynSolValue::Array(
            moduli_values.iter().map(|m| DynSolValue::Uint(*m, 256)).collect()  
        ),
    ]);
    
    params_tuple.abi_encode_params()
}
