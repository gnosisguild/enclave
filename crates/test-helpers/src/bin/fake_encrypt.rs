// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// This is a test script designed to encrypt some fixed data to a fhe public key
use anyhow::Result;
use clap::Parser;
use e3_sdk::bfv_helpers::{
    build_bfv_params_arc, decode_bfv_params, encode_ciphertexts, params::SET_2048_1032193_1,
};
use fhe::bfv::{Encoding, Plaintext, PublicKey};
use fhe_traits::{DeserializeParametrized, FheEncoder, FheEncrypter, Serialize};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::{fs, sync::Arc};

#[derive(Debug, Clone)]
struct HexBytes(pub Vec<u8>);

fn parse_hex(s: &str) -> Result<HexBytes, String> {
    // Remove "0x" or "0X" prefix if present
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    // Decode hex string to bytes
    hex::decode(s)
        .map(HexBytes)
        .map_err(|e| format!("Invalid hex string: {}", e))
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    output: String,

    #[arg(short, long, value_delimiter = ',')]
    plaintext: Vec<u64>,

    #[arg(long, value_parser = parse_hex)]
    params: Option<HexBytes>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Loading public key from {}", args.input);

    let bytes = fs::read(&args.input)?;
    let params = if let Some(params_bytes) = args.params {
        Arc::new(decode_bfv_params(&params_bytes.0))
    } else {
        let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
        build_bfv_params_arc(degree, plaintext_modulus, &moduli)
    };

    let pubkey = PublicKey::from_bytes(&bytes, &params)?;

    let raw_plaintext = vec![args.plaintext];

    println!("Encrypting plaintext: {:?}", raw_plaintext);
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let ciphertexts = raw_plaintext
        .into_iter()
        .map(|vec| {
            let pt = Plaintext::try_encode(&vec, Encoding::poly(), &params)?;
            Ok(pubkey.try_encrypt(&pt, &mut rng)?)
        })
        .collect::<Result<Vec<_>>>()?;

    let ciphertext_bytes = encode_ciphertexts(&ciphertexts);
    fs::write(&args.output, &ciphertext_bytes)?;
    println!("Created {}", args.output);

    Ok(())
}
