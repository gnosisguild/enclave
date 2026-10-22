// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! PRF witness values use the released FHE implementation and the circuit's key commitments.

use crate::commitments::compute_commitments;
use crate::WitnessError;
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use fhe::bfv::Ciphertext;
use fhe::trbfv::{PartyPrfKeyTransport, PartyPrfKeys, PRF_KEY_LEN};
use num_bigint::{BigInt, Sign};

fn domain(label: &[u8]) -> [u8; 64] {
    let mut domain = [0; 64];
    domain[..label.len()].copy_from_slice(label);
    domain
}

pub fn key_field(key: &[u8]) -> String {
    field_integer(Fr::from_le_bytes_mod_order(key)).to_string()
}

fn field_integer(value: Fr) -> BigInt {
    BigInt::from_bytes_le(Sign::Plus, &value.into_bigint().to_bytes_le())
}

pub fn key_commitment(recipient: u32, key: &[u8]) -> BigInt {
    field_integer(
        compute_commitments(
            vec![Fr::from(recipient), Fr::from_le_bytes_mod_order(key)],
            domain(b"PRF_KEY_FIELD"),
            [0x8000_0002, 1],
        )[0],
    )
}

/// Rehydrate application keys without changing the released byte/field encoding.
pub fn party_keys(
    party_idx: u32,
    n: usize,
    outgoing: &[Vec<u8>],
    incoming: &[Vec<u8>],
) -> Result<PartyPrfKeys, WitnessError> {
    let pad = |keys: &[Vec<u8>]| -> Result<Vec<[u8; PRF_KEY_LEN]>, WitnessError> {
        if keys.len() != n {
            return Err(WitnessError::Other(
                "PRF key count does not match the committee".into(),
            ));
        }
        keys.iter()
            .map(|key| {
                if key.is_empty() || key.len() > PRF_KEY_LEN {
                    return Err(WitnessError::Other(
                        "PRF key length must be in 1..=32".into(),
                    ));
                }
                let mut padded = [0; PRF_KEY_LEN];
                padded[..key.len()].copy_from_slice(key);
                Ok(padded)
            })
            .collect()
    };
    Ok(PartyPrfKeys::from_transport(PartyPrfKeyTransport::new(
        party_idx as usize + 1,
        n,
        pad(outgoing)?,
        pad(incoming)?,
    )?)?)
}

pub fn mask(
    party_idx: u32,
    decryptors: &[u32],
    outgoing: &[Vec<u8>],
    incoming: &[Vec<u8>],
    ciphertext: &Ciphertext,
    n: usize,
) -> Result<fhe_math::rq::Poly<fhe_math::rq::PowerBasis>, WitnessError> {
    let keys = party_keys(party_idx, n, outgoing, incoming)?;
    Ok(keys.mask(
        &decryptors.iter().map(|id| *id as usize).collect::<Vec<_>>(),
        ciphertext,
    )?)
}
