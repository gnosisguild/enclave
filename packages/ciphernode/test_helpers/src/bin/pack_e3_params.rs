use clap::{command, Parser};
use e3_sdk::bfv::{build_bfv_params_arc, encode_bfv_params};
use std::{error::Error, num::ParseIntError, process};

fn parse_hex(arg: &str) -> Result<u64, ParseIntError> {
    let without_prefix = arg.trim_start_matches("0x");
    u64::from_str_radix(without_prefix, 16)
}

#[derive(Parser, Debug)]
#[command(author, version, about="Encodes BFV parameters whilst generating a random CRP", long_about = None)]
struct Args {
    #[arg(short, long, required = true, value_parser = parse_hex,value_delimiter = ',')]
    moduli: Vec<u64>,

    #[arg(short, long)]
    degree: u64,

    #[arg(short, long = "plaintext-modulus")]
    plaintext_modulus: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    if args.moduli.len() == 0 {
        println!("Parameter `--moduli` must include at least one value");
        process::exit(1);
    }

    let params = build_bfv_params_arc(args.degree as usize, args.plaintext_modulus, &args.moduli);
    let encoded = encode_bfv_params(&params);

    for byte in encoded {
        print!("{:02x}", byte);
    }

    Ok(())
}
