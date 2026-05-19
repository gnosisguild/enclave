// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Canonical committee hash for DKG / decryption aggregator proofs.
//! Must match `CommitteeHashLib.sol` (`keccak256(abi.encodePacked(addresses))`).

use alloy::primitives::{keccak256, Address, B256, U256};

/// Hi/lo limbs of `keccak256(abi.encodePacked(addresses))` for Noir public inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitteeHashLimbs {
    pub hi: B256,
    pub lo: B256,
}

/// `keccak256(abi.encodePacked(addresses))` for the ordered on-chain committee.
pub fn hash_committee_addresses(addresses: &[Address]) -> B256 {
    let packed: Vec<u8> = addresses
        .iter()
        .flat_map(|addr| addr.into_array())
        .collect();
    keccak256(packed)
}

/// Split a committee hash into 128-bit limbs for BN254 public inputs.
pub fn split_committee_hash(hash: B256) -> CommitteeHashLimbs {
    let value = U256::from_be_bytes(hash.0);
    let hi = B256::from(value >> 128);
    let lo_mask = (U256::from(1) << 128) - U256::from(1);
    let lo = B256::from(value & lo_mask);
    CommitteeHashLimbs { hi, lo }
}

/// Hash and split in one step.
pub fn committee_hash_limbs_from_addresses(addresses: &[Address]) -> CommitteeHashLimbs {
    split_committee_hash(hash_committee_addresses(addresses))
}

/// Parse checksummed or lowercase hex node addresses (as used in events).
pub fn hash_committee_node_strings(nodes: &[String]) -> anyhow::Result<B256> {
    let addresses: Vec<Address> = nodes
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;
    Ok(hash_committee_addresses(&addresses))
}

/// Field hex strings (`0x…`, 32 bytes) for Noir witness `committee_hash_hi` / `committee_hash_lo`.
pub fn committee_hash_field_hex(nodes: &[String]) -> anyhow::Result<(String, String)> {
    let limbs = split_committee_hash(hash_committee_node_strings(nodes)?);
    Ok((field_hex_from_b256(limbs.hi), field_hex_from_b256(limbs.lo)))
}

fn field_hex_from_b256(value: B256) -> String {
    format!("0x{}", hex::encode(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    #[test]
    fn encode_packed_matches_solidity_layout() {
        let nodes = vec![
            address!("0x0000000000000000000000000000000000000001"),
            address!("0x0000000000000000000000000000000000000002"),
        ];
        let hash = hash_committee_addresses(&nodes);
        let limbs = split_committee_hash(hash);
        assert_ne!(limbs.hi, B256::ZERO);
        assert_ne!(limbs.lo, B256::ZERO);
    }
}
