use std::sync::{Arc, Mutex};

use alloy::{sol, sol_types::SolValue};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use anyhow::Result;
use crate::{setup_crp_params, ParamsWithCrp};

sol! {
    struct EncodedBfvParams {
        uint64[] moduli;
        uint64 degree;
        uint64 plaintext_modulus;
    }
    struct EncodedBfvParamsWithCrp {
        uint64[] moduli;
        uint64 degree;
        uint64 plaintext_modulus;
        bytes crp;
    }
}

pub fn abi_encode_params(moduli: Vec<u64>, degree: u64, plaintext_modulus: u64) -> Vec<u8> {
    EncodedBfvParams::abi_encode(&EncodedBfvParams {
        moduli,
        degree,
        plaintext_modulus,
    })
}

pub fn abi_encode_params_crpgen(moduli: Vec<u64>, degree: u64, plaintext_modulus: u64) -> Vec<u8> {
    let ParamsWithCrp { crp_bytes, .. } = setup_crp_params(
        &moduli,
        degree as usize,
        plaintext_modulus,
        Arc::new(Mutex::new(ChaCha20Rng::from_entropy())),
    );

    EncodedBfvParamsWithCrp::abi_encode(&EncodedBfvParamsWithCrp {
        moduli: moduli.clone(),
        degree,
        plaintext_modulus,
        crp: crp_bytes.into(),
    })
}
pub enum DecodedParams {
    WithoutCrp(EncodedBfvParams),
    WithCrp(EncodedBfvParamsWithCrp),
}

pub fn decode_params(data: &[u8]) -> Result<DecodedParams> {
    if let Ok(decoded) = EncodedBfvParamsWithCrp::abi_decode(data, false) {
        Ok(DecodedParams::WithCrp(decoded))
    } else {
        let decoded = EncodedBfvParams::abi_decode(data, false)?;
        Ok(DecodedParams::WithoutCrp(decoded))
    }
}
