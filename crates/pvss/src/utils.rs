// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::errors::CodegenError;
use crate::types::{Configs, SecurityLevel, Template, Toml, Wrapper};
use e3_zk_helpers::utils::to_string_1d_vec;
use num_bigint::BigInt;
use serde_json;
use std::path::Path;

pub fn map_witness_2d_vector_to_json(values: &Vec<Vec<BigInt>>) -> Vec<serde_json::Value> {
    values
        .iter()
        .map(|value| {
            serde_json::json!({
                "coefficients": to_string_1d_vec(value)
            })
        })
        .collect()
}

pub fn get_security_level(lambda: usize) -> SecurityLevel {
    if lambda < 80 {
        SecurityLevel::INSECURE
    } else {
        SecurityLevel::PRODUCTION
    }
}

pub fn generate_wrapper(n_recursive_proofs: usize, n_public_inputs: usize) -> Wrapper {
    format!(
        r#"use bb_proof_verification::{{UltraHonkProof, UltraHonkVerificationKey, verify_ultrahonk_proof}};
use lib::math::commitments::compute_aggregation_commitment;

// Number of proofs.
pub global N_PROOFS: u32 = {};
/// Number of public inputs/outputs per proof.
pub global N_PUBLIC_INPUTS: u32 = {};

fn main(
    verification_key: UltraHonkVerificationKey,
    proofs: [UltraHonkProof; N_PROOFS],
    public_inputs: pub [[Field; N_PUBLIC_INPUTS]; N_PROOFS],
    key_hash: Field,
) -> pub Field {{
    for i in 0..N_PROOFS {{
        verify_ultrahonk_proof(verification_key, proofs[i], public_inputs[i], key_hash);
    }}

    let mut aggregated_public_inputs = Vec::new();

    for i in 0..N_PROOFS {{
        for j in 0..N_PUBLIC_INPUTS {{
            aggregated_public_inputs.push(public_inputs[i][j]);
        }}
    }}

    compute_aggregation_commitment(aggregated_public_inputs)
}}
"#,
        n_recursive_proofs, n_public_inputs
    )
}

pub fn write_toml(toml: &Toml, path: Option<&Path>) -> Result<(), CodegenError> {
    let toml_path = path.unwrap_or_else(|| Path::new("."));
    let toml_path = toml_path.join("Prover.toml");
    Ok(std::fs::write(toml_path, toml)?)
}

pub fn write_template(template: &Template, path: Option<&Path>) -> Result<(), CodegenError> {
    let template_path = path.unwrap_or_else(|| Path::new("."));
    let template_path = template_path.join("main.nr");
    Ok(std::fs::write(template_path, template)?)
}

pub fn write_configs(configs: &Configs, path: Option<&Path>) -> Result<(), CodegenError> {
    let configs_path = path.unwrap_or_else(|| Path::new("."));
    let configs_path = configs_path.join("configs.nr");
    Ok(std::fs::write(configs_path, configs)?)
}

pub fn write_wrapper(wrapper: &Wrapper, path: Option<&Path>) -> Result<(), CodegenError> {
    let wrapper_path = path.unwrap_or_else(|| Path::new("."));
    let wrapper_path = wrapper_path.join("wrapper.nr");
    Ok(std::fs::write(wrapper_path, wrapper)?)
}
