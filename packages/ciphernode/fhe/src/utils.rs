use super::SharedRng;
use alloy::dyn_abi::{DynSolType, DynSolValue};
use alloy::primitives::U256;
use anyhow::{Context, Result};
use fhe_rs::{
    bfv::{BfvParameters, BfvParametersBuilder},
    mbfv::CommonRandomPoly,
};
use fhe_traits::{Deserialize, Serialize};
use std::sync::Arc;

pub struct ParamsWithCrp {
    pub moduli: Vec<u64>,
    pub degree: usize,
    pub plaintext_modulus: u64,
    pub crp_bytes: Vec<u8>,
    pub params: Arc<BfvParameters>,
}

pub fn setup_crp_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
    rng: SharedRng,
) -> ParamsWithCrp {
    let params = setup_bfv_params(moduli, degree, plaintext_modulus);
    let crp = set_up_crp(params.clone(), rng);
    ParamsWithCrp {
        moduli: moduli.to_vec(),
        degree,
        plaintext_modulus,
        crp_bytes: crp.to_bytes(),
        params,
    }
}

pub fn setup_bfv_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
) -> Arc<BfvParameters> {
    BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build_arc()
        .unwrap()
}

pub fn encode_bfv_params(moduli: Vec<u64>, degree: u64, plaintext_modulus: u64) -> Vec<u8> {
    let degree_value = U256::from(degree);
    let plaintext_value = U256::from(plaintext_modulus);
    let moduli_values = moduli.iter().map(|&m| U256::from(m)).collect::<Vec<_>>();

    let params_tuple = DynSolValue::Tuple(vec![
        DynSolValue::Uint(degree_value, 256),
        DynSolValue::Uint(plaintext_value, 256),
        DynSolValue::Array(
            moduli_values
                .iter()
                .map(|m| DynSolValue::Uint(*m, 256))
                .collect(),
        ),
    ]);

    params_tuple.abi_encode_params()
}

pub fn decode_bfv_params(bytes: &[u8]) -> DynSolValue {
    let params_type = DynSolType::Tuple(vec![
        DynSolType::Uint(256),
        DynSolType::Uint(256),
        DynSolType::Array(Box::new(DynSolType::Uint(256))),
    ]);

    return params_type.abi_decode_params(&bytes).unwrap();
}

pub fn set_up_crp(params: Arc<BfvParameters>, rng: SharedRng) -> CommonRandomPoly {
    CommonRandomPoly::new(&params, &mut *rng.lock().unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::dyn_abi::DynSolValue;
    use alloy::hex;
    use alloy::primitives::U256;

    #[test]
    fn test_encode_decode_basic() {
        let moduli = vec![1234567890u64];
        let degree = 2048u64;
        let plaintext_modulus = 65537u64;
        let encoded = encode_bfv_params(moduli.clone(), degree, plaintext_modulus);
        let decoded = decode_bfv_params(&encoded);

        if let DynSolValue::Tuple(values) = decoded {
            if let DynSolValue::Uint(decoded_degree, _) = &values[0] {
                assert_eq!(decoded_degree, &U256::from(degree));
            } else {
                panic!("Expected Uint for degree");
            }

            if let DynSolValue::Uint(decoded_plaintext, _) = &values[1] {
                assert_eq!(decoded_plaintext, &U256::from(plaintext_modulus));
            } else {
                panic!("Expected Uint for plaintext modulus");
            }

            if let DynSolValue::Array(decoded_moduli) = &values[2] {
                assert_eq!(decoded_moduli.len(), moduli.len());

                if let DynSolValue::Uint(decoded_modulus, _) = &decoded_moduli[0] {
                    assert_eq!(decoded_modulus, &U256::from(moduli[0]));
                } else {
                    panic!("Expected Uint for modulus");
                }
            } else {
                panic!("Expected Array for moduli");
            }
        } else {
            panic!("Expected Tuple for decoded result");
        }
    }
}
