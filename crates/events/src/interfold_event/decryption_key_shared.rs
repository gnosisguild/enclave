// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, SignedProofPayload};
use actix::Message;
use alloy::primitives::{keccak256, Address, Signature, U256};
use alloy::signers::{local::PrivateKeySigner, SignerSync};
use alloy::sol_types::SolValue;
use anyhow::Result;
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Exchange #3: Each honest node shares its aggregated trBFV partial key shares
/// with all other honest nodes, together with C4 proofs of correct BFV decryption.
#[derive(Message, Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
#[derivative(Debug)]
pub struct DecryptionKeyShared {
    pub e3_id: E3id,
    /// The sender's party_id.
    pub party_id: u64,
    /// The sender's node address.
    pub node: String,
    /// ECDSA-signed C4a proof for verification and fault attribution.
    pub signed_sk_decryption_proof: SignedProofPayload,
    /// Whether this was received from the network.
    pub external: bool,
    /// Dealer signature over the E3, sender, node address and complete C4 bundle.
    pub signature: ArcBytes,
}

impl DecryptionKeyShared {
    pub fn sign(mut self, signer: &PrivateKeySigner) -> Result<Self> {
        let signature = signer.sign_hash_sync(&self.digest()?.into())?;
        self.signature = ArcBytes::from_bytes(&signature.as_bytes());
        Ok(self)
    }

    pub fn recover_address(&self) -> Result<Address> {
        let signature = Signature::try_from(&self.signature[..])?;
        Ok(signature.recover_address_from_prehash(&self.digest()?.into())?)
    }

    /// The transport origin changes on receipt and is deliberately not signed.
    pub fn digest(&self) -> Result<[u8; 32]> {
        Ok(keccak256(
            (
                keccak256("InterfoldDecryptionKeyShareV2(bytes32 e3IdHash,uint256 dealer,bytes32 nodeHash,bytes32 proofsHash)"),
                keccak256(bincode::serialize(&self.e3_id)?),
                U256::from(self.party_id),
                keccak256(self.node.as_bytes()),
                keccak256(bincode::serialize(&self.signed_sk_decryption_proof)?),
            ).abi_encode(),
        ).into())
    }
}

impl Display for DecryptionKeyShared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DecryptionKeyShared {{ e3_id: {}, party_id: {} }}",
            self.e3_id, self.party_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CircuitName, Proof, ProofPayload, ProofType};

    #[test]
    fn decryption_bundle_signature_binds_dealer_and_complete_c4_proof() {
        let signer = PrivateKeySigner::random();
        let message = DecryptionKeyShared {
            e3_id: E3id::new("1", 1),
            party_id: 0,
            node: signer.address().to_string(),
            signed_sk_decryption_proof: SignedProofPayload {
                payload: ProofPayload {
                    e3_id: E3id::new("1", 1),
                    proof_type: ProofType::C4aSkShareDecryption,
                    proof: Proof::new(
                        CircuitName::DkgShareDecryption,
                        ArcBytes::from_bytes(&[1]),
                        ArcBytes::from_bytes(&[2]),
                    ),
                },
                signature: ArcBytes::from_bytes(&[3; 65]),
            },
            external: false,
            signature: ArcBytes::default(),
        }
        .sign(&signer)
        .unwrap();
        assert_eq!(message.recover_address().unwrap(), signer.address());
        let tampers: [fn(&mut DecryptionKeyShared); 7] = [
            |m| m.e3_id = E3id::new("2", 1),
            |m| m.party_id += 1,
            |m| m.node = Address::ZERO.to_string(),
            |m| m.signed_sk_decryption_proof.payload.proof.data = ArcBytes::from_bytes(&[9]),
            |m| {
                m.signed_sk_decryption_proof.payload.proof.public_signals =
                    ArcBytes::from_bytes(&[9])
            },
            |m| m.signed_sk_decryption_proof.payload.proof_type = ProofType::C0PkBfv,
            |m| m.signed_sk_decryption_proof.signature = ArcBytes::from_bytes(&[9; 65]),
        ];
        for tamper in tampers {
            let mut changed = message.clone();
            tamper(&mut changed);
            assert_ne!(changed.recover_address().ok(), Some(signer.address()));
        }
        let mut received = message.clone();
        received.external = true;
        assert_eq!(received.recover_address().unwrap(), signer.address());
        received.signature = ArcBytes::default();
        assert!(received.recover_address().is_err());
    }
}
