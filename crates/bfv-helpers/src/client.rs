use crate::{build_bfv_params_arc, params::SET_2048_1032193_1};
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use fhe_rs::bfv::Encoding;
use fhe_rs::bfv::Plaintext;
use fhe_rs::bfv::PublicKey;
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use rand::thread_rng;

pub fn bfv_encrypt_u64(data: u64, public_key: Vec<u8>) -> Result<Vec<u8>> {
    let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
    let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

    let pk = PublicKey::from_bytes(&public_key, &params)
        .map_err(|e| anyhow!("Error deserializing public key:{e}"))?;

    let input = vec![data];
    let pt = Plaintext::try_encode(&input, Encoding::poly(), &params)
        .map_err(|e| anyhow!("Error encoding plaintext: {e}"))?;

    let ct = pk
        .try_encrypt(&pt, &mut thread_rng())
        .map_err(|e| anyhow!("Error encrypting data: {e}"))?;

    let encrypted_data = ct.to_bytes();
    Ok(encrypted_data)
}
