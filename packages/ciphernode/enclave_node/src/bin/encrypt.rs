use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use enclave_core::{setup_bfv_params, CiphertextSerializer};
use fhe::bfv::{Encoding, Plaintext, PublicKey};
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::fs;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read the base64 encoded string from the input file
    println!("Loading public key from {}", args.input);
    let encoded_string = fs::read_to_string(&args.input)?;

    // Decode the base64 string
    let decoded_bytes: Vec<u8> = general_purpose::STANDARD.decode(encoded_string.trim())?;
    let params = setup_bfv_params(&[0x3FFFFFFF000001], 2048, 1032193);
    let pubkey = PublicKey::from_bytes(&decoded_bytes, &params)?;

    let yes = 1234u64;
    let no = 873827u64;
    let raw_plaintext = vec![yes, no];
    println!("Encrypting plaintext: {:?}", raw_plaintext);

    let pt = Plaintext::try_encode(&raw_plaintext, Encoding::poly(), &params)?;
    let ciphertext = pubkey.try_encrypt(&pt, &mut ChaCha20Rng::seed_from_u64(42))?;
    let ciphertext_bytes = CiphertextSerializer::to_bytes(ciphertext.clone(), params.clone())?;
   
    fs::write(&args.output, &ciphertext_bytes)?;
    println!("Created {}", args.output);

    Ok(())
}
