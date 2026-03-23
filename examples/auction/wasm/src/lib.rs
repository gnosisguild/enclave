use auction_example::{build_params, encode_bid};
use fhe::bfv::PublicKey;
use fhe_traits::{DeserializeParametrized, FheEncrypter, Serialize};
use rand::rngs::OsRng;
use wasm_bindgen::prelude::*;

/// Encrypt a bid value client-side using the auction's public key.
///
/// Returns the ciphertext as raw bytes (caller should base64-encode for the API).
#[wasm_bindgen]
pub fn encrypt_bid(pk_bytes: &[u8], bid: u64) -> Result<Vec<u8>, JsValue> {
    let params = build_params();

    let pk = PublicKey::from_bytes(pk_bytes, &params)
        .map_err(|e| JsValue::from_str(&format!("bad public key: {e}")))?;

    let pt = encode_bid(bid, &params);

    let ct = pk
        .try_encrypt(&pt, &mut OsRng)
        .map_err(|e| JsValue::from_str(&format!("encryption failed: {e}")))?;

    Ok(ct.to_bytes())
}
