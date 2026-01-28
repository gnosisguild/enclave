// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::circuits::pk_bfv::circuit::PkBfvCircuit;
use crate::circuits::pk_bfv::computation::{Bits, Bounds, Witness};
use crate::errors::CodegenError;
use crate::traits::Circuit;
use crate::traits::Computation;
use crate::traits::ReduceToZkpModulus;
use crate::types::Artifacts;
use crate::types::{Configs, Template, Toml, Wrapper};
use crate::utils::generate_wrapper;
use crate::utils::get_security_level;
use crate::utils::map_witness_2d_vector_to_json;
use crate::utils::write_configs;
use crate::utils::write_template;
use crate::utils::write_toml;
use crate::utils::write_wrapper;
use e3_fhe_params::BfvParamSet;
use e3_fhe_params::BfvPreset;
use fhe::bfv::BfvParameters;
use fhe::bfv::PublicKey;
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlJson {
    pub pk0is: Vec<serde_json::Value>,
    pub pk1is: Vec<serde_json::Value>,
}

pub fn generate_toml(witness: Witness) -> Result<Toml, CodegenError> {
    let pk0is = map_witness_2d_vector_to_json(&witness.pk0is);
    let pk1is = map_witness_2d_vector_to_json(&witness.pk1is);

    let toml_json = TomlJson { pk0is, pk1is };
    Ok(toml::to_string(&toml_json)?)
}

pub fn codegen(preset: BfvPreset, public_key: PublicKey) -> Result<Artifacts, CodegenError> {
    let params = BfvParamSet::from(preset).build_arc();
    // Compute.
    let bounds = Bounds::compute(&params, &())?;
    let bits = Bits::compute(&params, &bounds)?;
    let witness = Witness::compute(&params, &public_key)?;
    let zkp_witness = witness.reduce_to_zkp_modulus();

    let toml = generate_toml(zkp_witness)?;
    let configs = generate_configs(&params, &bits);
    let template = generate_template(preset.metadata().lambda);
    let wrapper = generate_wrapper(
        <PkBfvCircuit as Circuit>::N_PROOFS,
        <PkBfvCircuit as Circuit>::N_PUBLIC_INPUTS,
    );

    Ok(Artifacts {
        toml,
        configs,
        template,
        wrapper,
    })
}

pub fn generate_template(lambda: usize) -> Template {
    format!(
        r#"use lib::configs::{}::bfv::{{L, N, {}_BIT_PK}};
use lib::core::bfv_pk::BfvPkCommit;
use lib::math::polynomial::Polynomial;

fn main(pk0is: [Polynomial<N>; L], pk1is: [Polynomial<N>; L]) -> pub Field {{
    let pk_bfv: BfvPkCommit<N, L, {}_BIT_PK> = BfvPkCommit::new(pk0is, pk1is);
    pk_bfv.verify()
}}
"#,
        get_security_level(lambda).as_str(),
        <PkBfvCircuit as Circuit>::PREFIX,
        <PkBfvCircuit as Circuit>::PREFIX,
    )
}

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

pub fn write_artifacts(
    toml: &Toml,
    template: &Template,
    configs: &Configs,
    wrapper: &Wrapper,
    path: Option<&Path>,
) -> Result<(), CodegenError> {
    write_toml(&toml, path)?;
    write_template(&template, path)?;
    write_configs(&configs, path)?;
    write_wrapper(&wrapper, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use e3_fhe_params::{BfvParamSet, BfvPreset};
    use tempfile::TempDir;

    #[test]
    fn test_toml_generation_and_structure() {
        let preset = BfvPreset::InsecureThresholdBfv512;
        let params = BfvParamSet::from(preset).build_arc();
        let sample = sample::generate_sample(&params);
        let artifacts = codegen(preset, sample.public_key).unwrap();

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
            &artifacts.toml,
            &artifacts.template,
            &artifacts.configs,
            &artifacts.wrapper,
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

        let template_path = temp_dir.path().join("main.nr");
        assert!(template_path.exists());

        let template_content = std::fs::read_to_string(&template_path).unwrap();
        assert!(template_content.contains("pk0is: [Polynomial<N>; L],"));
        assert!(template_content.contains("pk1is: [Polynomial<N>; L]"));

        let wrapper_path = temp_dir.path().join("wrapper.nr");
        assert!(wrapper_path.exists());

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
