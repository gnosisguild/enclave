// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::SharedRng;
use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use anyhow::{Context, Result};
use rand::Rng;
use std::str::FromStr;

pub fn rand_eth_addr(rng: &SharedRng) -> String {
    {
        let rnum = &mut rng.lock().unwrap().gen::<[u8; 20]>();
        Address::from_slice(rnum).to_string()
    }
}

pub fn eth_address_from_private_key(private_key: &str) -> Result<Address> {
    let signer = PrivateKeySigner::from_str(private_key).context("invalid private key")?;
    Ok(signer.address())
}
