use super::SharedRng;
use alloy::dyn_abi::{DynSolType, DynSolValue};
use alloy::primitives::U256;
use anyhow::anyhow;
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

pub fn encode_bfv_parameters(degree: u64, plaintext_modulus: u64, moduli: Vec<u64>) -> Vec<u8> {
    DynSolValue::Tuple(vec![
        DynSolValue::Uint(U256::from(degree), 256),
        DynSolValue::Uint(U256::from(plaintext_modulus), 256),
        DynSolValue::Array(
            moduli
                .iter()
                .map(|&m| U256::from(m))
                .collect::<Vec<_>>()
                .iter()
                .map(|m| DynSolValue::Uint(*m, 256))
                .collect(),
        ),
    ])
    .abi_encode_params()
}

pub fn decode_bfv_parameters(bytes: &[u8]) -> Result<(u64, u64, Vec<u64>)> {
    let params_type = DynSolType::Tuple(vec![
        DynSolType::Uint(256),
        DynSolType::Uint(256),
        DynSolType::Array(Box::new(DynSolType::Uint(256))),
    ]);

    let decoded = params_type.abi_decode_params(bytes)?;

    if let DynSolValue::Tuple(values) = decoded {
        // Extract degree
        let degree = if let DynSolValue::Uint(val, _) = &values[0] {
            val.to::<u64>()
        } else {
            return Err(anyhow!("Expected Uint for degree"));
        };

        // Extract plaintext_modulus
        let plaintext_modulus = if let DynSolValue::Uint(val, _) = &values[1] {
            val.to::<u64>()
        } else {
            return Err(anyhow!("Expected Uint for plaintext modulus"));
        };

        // Extract moduli
        let moduli = if let DynSolValue::Array(decoded_moduli) = &values[2] {
            decoded_moduli
                .iter()
                .map(|v| {
                    if let DynSolValue::Uint(val, _) = v {
                        Ok(val.to::<u64>())
                    } else {
                        Err(anyhow!("Expected Uint for modulus"))
                    }
                })
                .collect::<Result<Vec<u64>>>()?
        } else {
            return Err(anyhow!("Expected Array for moduli"));
        };

        Ok((degree, plaintext_modulus, moduli))
    } else {
        Err(anyhow!("Expected Tuple for decoded result"))
    }
}

pub fn set_up_crp(params: Arc<BfvParameters>, rng: SharedRng) -> CommonRandomPoly {
    CommonRandomPoly::new(&params, &mut *rng.lock().unwrap()).unwrap()
}
