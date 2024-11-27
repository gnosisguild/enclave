use crate::ciphertext_output::ComputeResult;
use crate::merkle_tree::MerkleTree;
use sha3::{Digest, Keccak256};

pub type FHEProcessor = fn(&FHEInputs) -> Vec<u8>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FHEInputs {
    pub ciphertexts: Vec<(Vec<u8>, u64)>,
    pub params: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ComputeInput {
    pub fhe_inputs: FHEInputs,
    pub ciphertext_hash: Vec<u8>,
    pub leaf_hashes: Vec<String>,
}

impl ComputeInput {
    pub fn process(&self, fhe_processor: FHEProcessor) -> ComputeResult {
        let processed_ciphertext = (fhe_processor)(&self.fhe_inputs);
        let processed_hash = Keccak256::digest(&processed_ciphertext).to_vec();
        let params_hash = Keccak256::digest(&self.fhe_inputs.params).to_vec();

        assert_eq!(processed_hash, self.ciphertext_hash, "Ciphertext hash mismatch");

        let merkle_root = MerkleTree {
            leaf_hashes: self.leaf_hashes.clone(),
        }
        .build_tree()
        .root()
        .unwrap();

        ComputeResult {
            ciphertext_hash: processed_hash,
            params_hash,
            merkle_root: hex::decode(merkle_root).unwrap(),
        }
    }
}
