// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{Error, Result};
use bincode::deserialize;
use e3_compute_provider::{ComputeInput, ComputeResult};
use e3_user_program::fhe_processor;
use risc0_zkvm::guest::env;
use std::io::Read;

fn decode_input(input: &[u8]) -> Result<Vec<u8>, Error> {
    Ok(risc0_zkvm::serde::from_slice(input)?)
}

fn main() {
    let mut input_slice = Vec::<u8>::new();
    env::stdin().read_to_end(&mut input_slice).unwrap();
    let input: ComputeInput = deserialize(&decode_input(&input_slice).unwrap()).unwrap();

    let result: ComputeResult = input.process(fhe_processor);

    env::commit(&result);
}
