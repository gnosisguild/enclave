// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Code generation for the public-key BFV circuit: Prover.toml and configs.nr.

use crate::circuits::pk_bfv::circuit::PkBfvCircuitInput;
use crate::circuits::pk_bfv::computation::{Bits, Bounds, Witness};
use crate::circuits::PkBfvCircuit;
use crate::codegen::Artifacts;
use crate::codegen::CircuitCodegen;
use crate::computation::Computation;
use crate::computation::Configs;
use crate::computation::ReduceToZkpModulus;
use crate::computation::Toml;
use crate::errors::CircuitsErrors;
use crate::registry::Circuit;
use crate::utils::map_witness_2d_vector_to_json;
use fhe::bfv::BfvParameters;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;

/// Implementation of [`CircuitCodegen`] for [`PkBfvCircuit`].
impl CircuitCodegen for PkBfvCircuit {
    type Params = Arc<BfvParameters>;
    type Input = PkBfvCircuitInput;
    type Error = CircuitsErrors;

    fn codegen(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<Artifacts, Self::Error> {
        // Compute.
        let bounds = Bounds::compute(&params, &())?;
        let bits = Bits::compute(&params, &bounds)?;
        let witness = Witness::compute(&params, input)?;
        let zkp_witness = witness.reduce_to_zkp_modulus();

        let toml = generate_toml(zkp_witness)?;
        let configs = generate_configs(&params, &bits);

        Ok(Artifacts { toml, configs })
    }
}

/// JSON-serializable structure for Prover.toml (pk0is, pk1is arrays).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlJson {
    /// First component of the public key per modulus (for the prover).
    pub pk0is: Vec<serde_json::Value>,
    /// Second component of the public key per modulus (for the prover).
    pub pk1is: Vec<serde_json::Value>,
}

/// Builds the Prover TOML string from the pk-bfv witness (pk0is, pk1is).
pub fn generate_toml(witness: Witness) -> Result<Toml, CircuitsErrors> {
    let pk0is = map_witness_2d_vector_to_json(&witness.pk0is);
    let pk1is = map_witness_2d_vector_to_json(&witness.pk1is);

    let toml_json = TomlJson { pk0is, pk1is };
    Ok(toml::to_string(&toml_json)?)
}

/// Builds the configs.nr string (N, L, bit parameters) for the Noir prover.
pub fn generate_configs(params: &Arc<BfvParameters>, bits: &Bits) -> Configs {
    format!(
        r#"// Global configs for Public Key BFV circuit
pub global N: u32 = {};
pub global L: u32 = {};

/************************************
-------------------------------------
pk_bfv (CIRCUIT 0 - PUBLIC KEY BFV COMMITMENT)
-------------------------------------
************************************/

// pk_bfv - bit parameters
pub global {}_BIT_PK: u32 = {};
"#,
        params.degree(),
        params.moduli().len(),
        <PkBfvCircuit as Circuit>::PREFIX,
        bits.pk_bit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::write_artifacts;
    use crate::sample::Sample;
    use e3_fhe_params::BfvParamSet;
    use e3_fhe_params::DEFAULT_BFV_PRESET;
    use tempfile::TempDir;

    #[test]
    fn test_toml_generation_and_structure() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let sample = Sample::generate(&params);
        let artifacts = PkBfvCircuit
            .codegen(
                &params,
                &PkBfvCircuitInput {
                    public_key: sample.public_key,
                },
            )
            .unwrap();

        let parsed: toml::Value = artifacts.toml.parse().unwrap();
        let pk0is = parsed
            .get("pk0is")
            .and_then(|value| value.as_array())
            .unwrap();
        let pk1is = parsed
            .get("pk1is")
            .and_then(|value| value.as_array())
            .unwrap();
        assert!(!pk0is.is_empty());
        assert!(!pk1is.is_empty());

        let temp_dir = TempDir::new().unwrap();
        write_artifacts(
            Some(&artifacts.toml),
            &artifacts.configs,
            Some(temp_dir.path()),
        )
        .unwrap();

        let output_path = temp_dir.path().join("Prover.toml");
        assert!(output_path.exists());

        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("pk0is"));
        assert!(content.contains("pk1is"));

        assert!(artifacts.toml.contains("[[pk0is]]"));
        assert!(artifacts.toml.contains("[[pk1is]]"));

        let configs_path = temp_dir.path().join("configs.nr");
        assert!(configs_path.exists());

        let configs_content = std::fs::read_to_string(&configs_path).unwrap();
        let bounds = Bounds::compute(&params, &()).unwrap();
        let bits = Bits::compute(&params, &bounds).unwrap();
        assert!(configs_content.contains(format!("N: u32 = {}", params.degree()).as_str()));
        assert!(configs_content.contains(format!("L: u32 = {}", params.moduli().len()).as_str()));
        assert!(configs_content.contains(format!("PK_BFV_BIT_PK: u32 = {}", bits.pk_bit).as_str()));
    }
}
