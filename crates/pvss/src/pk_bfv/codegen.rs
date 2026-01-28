// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::pk_bfv::computation::*;
use crate::traits::Computation;
use crate::traits::ReduceToZkpModulus;
use crate::utils::map_witness_2d_vector_to_json;
use crate::CiphernodesCommitteeSize;
use fhe::bfv::BfvParameters;
use fhe::bfv::PublicKey;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Toml {
    pub pk0is: Vec<serde_json::Value>,
    pub pk1is: Vec<serde_json::Value>,
}

pub fn generate_toml(witness: Witness) -> Toml {
    let pk0is = map_witness_2d_vector_to_json(&witness.pk0is);
    let pk1is = map_witness_2d_vector_to_json(&witness.pk1is);

    Toml { pk0is, pk1is }
}

pub fn codegen(
    committee: CiphernodesCommitteeSize,
    params: Arc<BfvParameters>,
    public_key: PublicKey,
) -> Toml {
    // Compute.
    let bounds = Bounds::compute(&params, &()).unwrap();
    let bits = Bits::compute(&params, &bounds).unwrap();
    let witness = Witness::compute(&params, &public_key).unwrap();
    let zkp_witness = witness.reduce_to_zkp_modulus();

    // Generate Prover.toml
    let toml = generate_toml(zkp_witness);
    // Generate configs.nr
    // Generate main.nr
    // Generate wrapper.nr

    toml
}

pub fn write_artifacts(toml: Toml, path: Option<&Path>) {
    let toml_path = path.unwrap_or_else(|| Path::new("."));
    let toml_path = toml_path.join("Prover.toml");
    let toml_content = toml::to_string(&toml).unwrap();
    std::fs::write(toml_path, toml_content).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use e3_fhe_params::{BfvParamSet, BfvPreset};
    use tempfile::TempDir;

    #[test]
    fn test_toml_generation_and_structure() {
        let committee = CiphernodesCommitteeSize::Small;
        let params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let sample = sample::generate_sample(&params);
        let toml = codegen(committee, params, sample.public_key);

        assert!(toml.pk0is.len() > 0);
        assert!(toml.pk1is.len() > 0);

        let temp_dir = TempDir::new().unwrap();
        write_artifacts(toml.clone(), Some(temp_dir.path()));

        let output_path = temp_dir.path().join("Prover.toml");
        assert!(output_path.exists());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("pk0is"));
        assert!(content.contains("pk1is"));

        let toml_string = toml::to_string(&toml).unwrap();
        assert!(toml_string.contains("[[pk0is]]"));
        assert!(toml_string.contains("[[pk1is]]"));
    }
}
