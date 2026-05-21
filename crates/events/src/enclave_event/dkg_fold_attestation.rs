// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! ECDSA attestation binding a node's [`CircuitName::NodeFold`] output to its committee address.
//!
//! Used to close the attribution gap between `party_ids[i]` in the DKG aggregator and the
//! operator that produced fold row `i`: the aggregator cannot permute folds without valid
//! signatures from each claimed party.

use crate::E3id;
use alloy::primitives::{keccak256, Address, U256};
use alloy::signers::{local::PrivateKeySigner, SignerSync};
use alloy::sol_types::SolValue;
use anyhow::{anyhow, Result};
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};

/// Commitments surfaced from a [`CircuitName::NodeFold`] proof (C4 aggregate outputs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DkgFoldAggCommits {
    pub sk_agg_commit: [u8; 32],
    pub esm_agg_commit: [u8; 32],
}

/// Payload signed after `NodeDkgFold` completes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DkgFoldAttestationPayload {
    pub e3_id: E3id,
    /// Sortition / committee slot id (index into on-chain `topNodes` when ids are dense).
    pub party_id: u64,
    pub agg_commits: DkgFoldAggCommits,
}

impl DkgFoldAttestationPayload {
    /// Must match `DKG_FOLD_ATTESTATION_TYPEHASH` in `CiphernodeRegistryOwnable.sol` when added.
    pub fn typehash() -> [u8; 32] {
        keccak256(
            "DkgFoldAttestation(uint256 chainId,uint256 e3Id,uint256 partyId,bytes32 skAggCommit,bytes32 esmAggCommit)",
        )
        .into()
    }

    pub fn digest(&self) -> Result<[u8; 32]> {
        let e3_id_u256: U256 = self
            .e3_id
            .clone()
            .try_into()
            .map_err(|_| anyhow!("E3id cannot be converted to U256"))?;
        let encoded = (
            Self::typehash(),
            U256::from(self.e3_id.chain_id()),
            e3_id_u256,
            U256::from(self.party_id),
            self.agg_commits.sk_agg_commit,
            self.agg_commits.esm_agg_commit,
        )
            .abi_encode();
        Ok(keccak256(&encoded).into())
    }
}

/// `eth_sign` over [`DkgFoldAttestationPayload::digest`].
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct SignedDkgFoldAttestation {
    pub payload: DkgFoldAttestationPayload,
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub signature: ArcBytes,
}

impl SignedDkgFoldAttestation {
    pub fn sign(payload: DkgFoldAttestationPayload, signer: &PrivateKeySigner) -> Result<Self> {
        let digest = payload.digest()?;
        let sig = signer
            .sign_message_sync(&digest)
            .map_err(|e| anyhow!("Failed to sign DkgFoldAttestation: {e}"))?;
        Ok(Self {
            payload,
            signature: ArcBytes::from_bytes(&sig.as_bytes()),
        })
    }

    pub fn recover_address(&self) -> Result<Address> {
        use alloy::primitives::Signature;
        let sig = Signature::try_from(&self.signature[..])
            .map_err(|e| anyhow!("Invalid DkgFoldAttestation signature: {e}"))?;
        let digest = self.payload.digest()?;
        sig.recover_address_from_msg(&digest)
            .map_err(|e| anyhow!("Failed to recover DkgFoldAttestation signer: {e}"))
    }

    pub fn verify_signer(&self, expected: &Address) -> Result<bool> {
        Ok(self.recover_address()? == *expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::local::PrivateKeySigner;

    #[test]
    fn sign_and_recover_roundtrip() {
        let signer: PrivateKeySigner =
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .parse()
                .unwrap();
        let payload = DkgFoldAttestationPayload {
            e3_id: E3id::new("0", 1),
            party_id: 1,
            agg_commits: DkgFoldAggCommits {
                sk_agg_commit: [7u8; 32],
                esm_agg_commit: [9u8; 32],
            },
        };
        let signed = SignedDkgFoldAttestation::sign(payload, &signer).unwrap();
        assert_eq!(signed.recover_address().unwrap(), signer.address());
    }
}
