// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// This is a test script designed to encrypt some fixed data to a fhe public key
use clap::Parser;
use e3_sdk::bfv_helpers::{build_bfv_params_from_set_arc, params::SET_2048_1032193_1};
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

    #[arg(short, long, value_delimiter = ',')]
    plaintext: Vec<u64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Read the base64 encoded string from the input file
    println!("Loading public key from {}", args.input);
    let bytes = fs::read(&args.input)?;

    // Decode the base64 string
    let param_set = SET_2048_1032193_1;
    let params = build_bfv_params_from_set_arc(param_set);
    let pubkey = PublicKey::from_bytes(&bytes, &params)?;

    let raw_plaintext = args.plaintext;
    println!("Encrypting plaintext: {:?}", raw_plaintext);

    let pt = Plaintext::try_encode(&raw_plaintext, Encoding::poly(), &params)?;
    let ciphertext = pubkey.try_encrypt(&pt, &mut ChaCha20Rng::seed_from_u64(42))?;
    let ciphertext_bytes = ciphertext.clone().to_bytes();

    fs::write(&args.output, &ciphertext_bytes)?;
    println!("Created {}", args.output);

    Ok(())
}
