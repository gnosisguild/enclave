use base64::{engine::general_purpose, Engine as _};
use enclave_core::{setup_bfv_params, setup_crp_params, CiphertextSerializer, ParamsWithCrp};
use fhe::bfv::{Encoding, Plaintext, PublicKey};
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use rand::SeedableRng;
use rand_chacha::{rand_core::OsRng, ChaCha20Rng};
use std::{fs, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the base64 encoded string from a file
    println!("Loading public key from ./scripts/pubkey.b64");
    let encoded_string = fs::read_to_string("scripts/pubkey.b64")?;

    // Decode the base64 string
    let decoded_bytes: Vec<u8> = general_purpose::STANDARD.decode(encoded_string.trim())?;
    let params = setup_bfv_params(&[0x3FFFFFFF000001], 2048, 1032193);
    let pubkey = PublicKey::from_bytes(&decoded_bytes, &params)?;

    let yes = 1234u64;
    let no = 873827u64;

    let raw_plaintext = vec![yes, no];
    println!("Encrypting plaintext: {:?}", raw_plaintext);
    // let expected_raw_plaintext = bincode::serialize(&raw_plaintext)?;

    let pt = Plaintext::try_encode(&raw_plaintext, Encoding::poly(), &params)?;

    let ciphertext = pubkey.try_encrypt(&pt, &mut ChaCha20Rng::seed_from_u64(42))?;
    let ciphertext_bytes = CiphertextSerializer::to_bytes(ciphertext.clone(), params.clone())?;

    let encrypted = general_purpose::STANDARD.encode(ciphertext_bytes);
    fs::write("scripts/encrypted.b64", &encrypted).unwrap();
    println!("Created ./scripts/encrypted.b64");
    Ok(())
}
