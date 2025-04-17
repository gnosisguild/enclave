use alloy::dyn_abi::{DynSolType, DynSolValue};
use alloy::primitives::U256;
use anyhow::*;
#[allow(dead_code)]
use fhe::bfv::{BfvParameters, BfvParametersBuilder};
use std::{error::Error as StdError, sync::Arc};

pub fn generate_bfv_parameters() -> Result<Arc<BfvParameters>, Box<dyn StdError + Send + Sync>> {
    BfvParametersBuilder::new()
        .set_degree(2048)
        .set_plaintext_modulus(1032193)
        .set_moduli(&[0xffffffff00001])
        .build_arc()
        .map_err(|e| Box::new(e) as Box<dyn StdError + Send + Sync>)
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

pub fn decode_bfv_parameters(bytes: &[u8]) -> Result<(u64, u64, Vec<u64>), anyhow::Error> {
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
                .collect::<Result<Vec<u64>, anyhow::Error>>()?
        } else {
            return Err(anyhow!("Expected Array for moduli"));
        };

        Ok((degree, plaintext_modulus, moduli))
    } else {
        Err(anyhow!("Expected Tuple for decoded result"))
    }
}
