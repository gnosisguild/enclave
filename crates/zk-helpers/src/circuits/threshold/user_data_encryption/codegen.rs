// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Code generation for the public-key BFV circuit: Prover.toml and configs.nr.

use crate::circuits::computation::Computation;
use crate::crt_polynomial_to_toml_json;
use crate::polynomial_to_toml_json;
use crate::threshold::user_data_encryption::circuit::UserDataEncryptionCircuit;
use crate::threshold::user_data_encryption::computation::{Configs, Witness};
use crate::threshold::user_data_encryption::UserDataEncryptionCircuitInput;
use crate::utils::join_display;
use crate::Circuit;
use crate::CircuitCodegen;
use crate::CircuitsErrors;
use crate::{Artifacts, CodegenConfigs, CodegenToml};

use e3_fhe_params::BfvPreset;
use serde::{Deserialize, Serialize};
use serde_json;

/// Implementation of [`CircuitCodegen`] for [`UserDataEncryptionCircuit`].
impl CircuitCodegen for UserDataEncryptionCircuit {
    type BfvThresholdParametersPreset = BfvPreset;
    type Input = UserDataEncryptionCircuitInput;
    type Error = CircuitsErrors;

    fn codegen(
        &self,
        preset: Self::BfvThresholdParametersPreset,
        input: &Self::Input,
    ) -> Result<Artifacts, Self::Error> {
        let witness = Witness::compute(preset, input)?;
        let configs = Configs::compute(preset, &())?;

        let toml = generate_toml(witness)?;
        let configs = generate_configs(preset, &configs);

        Ok(Artifacts { toml, configs })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlJson {
    pub pk0is: Vec<serde_json::Value>,
    pub pk1is: Vec<serde_json::Value>,
    pub ct0is: Vec<serde_json::Value>,
    pub ct1is: Vec<serde_json::Value>,
    pub u: serde_json::Value,
    pub e0: serde_json::Value,
    pub e0is: Vec<serde_json::Value>,
    pub e0_quotients: Vec<serde_json::Value>,
    pub e1: serde_json::Value,
    pub k1: serde_json::Value,
    pub r1is: Vec<serde_json::Value>,
    pub r2is: Vec<serde_json::Value>,
    pub p1is: Vec<serde_json::Value>,
    pub p2is: Vec<serde_json::Value>,
    pub pk_commitment: String,
}

pub fn generate_toml(witness: Witness) -> Result<CodegenToml, CircuitsErrors> {
    let pk0is = crt_polynomial_to_toml_json(&witness.pk0is);
    let pk1is = crt_polynomial_to_toml_json(&witness.pk1is);
    let ct0is = crt_polynomial_to_toml_json(&witness.ct0is);
    let ct1is = crt_polynomial_to_toml_json(&witness.ct1is);
    let u = polynomial_to_toml_json(&witness.u);
    let e0 = polynomial_to_toml_json(&witness.e0);
    let e0is = crt_polynomial_to_toml_json(&witness.e0is);
    let e0_quotients = crt_polynomial_to_toml_json(&witness.e0_quotients);
    let e1 = polynomial_to_toml_json(&witness.e1);
    let k1 = polynomial_to_toml_json(&witness.k1);
    let r1is = crt_polynomial_to_toml_json(&witness.r1is);
    let r2is = crt_polynomial_to_toml_json(&witness.r2is);
    let p1is = crt_polynomial_to_toml_json(&witness.p1is);
    let p2is = crt_polynomial_to_toml_json(&witness.p2is);
    let pk_commitment = witness.pk_commitment.to_string();

    let toml_json = TomlJson {
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e0is,
        e0_quotients,
        e1,
        k1,
        r1is,
        r2is,
        p1is,
        p2is,
        pk_commitment,
    };

    Ok(toml::to_string(&toml_json)?)
}

pub fn generate_configs(preset: BfvPreset, configs: &Configs) -> CodegenConfigs {
    let prefix = <UserDataEncryptionCircuit as Circuit>::PREFIX;

    let qis_str = join_display(&configs.moduli, ", ");
    let k0is_str = join_display(&configs.k0is, ", ");
    let pk_bounds_str = join_display(&configs.bounds.pk_bounds, ", ");
    let r1_low_bounds_str = join_display(&configs.bounds.r1_low_bounds, ", ");
    let r1_up_bounds_str = join_display(&configs.bounds.r1_up_bounds, ", ");
    let r2_bounds_str = join_display(&configs.bounds.r2_bounds, ", ");
    let p1_bounds_str = join_display(&configs.bounds.p1_bounds, ", ");
    let p2_bounds_str = join_display(&configs.bounds.p2_bounds, ", ");

    format!(
        r#"use crate::core::threshold::user_data_encryption::Configs as UserDataEncryptionConfigs;

// Global configs for User Data Encryption circuit
pub global N: u32 = {};
pub global L: u32 = {};
pub global QIS: [Field; L] = [{}];

/************************************
-------------------------------------
user_data_encryption (USED FOR DATA ENCRYPTION)
-------------------------------------
************************************/

pub global {}_BIT_PK: u32 = {};
pub global {}_BIT_CT: u32 = {};
pub global {}_BIT_U: u32 = {};
pub global {}_BIT_E0: u32 = {};
pub global {}_BIT_E1: u32 = {};
pub global {}_BIT_K: u32 = {};
pub global {}_BIT_R1: u32 = {};
pub global {}_BIT_R2: u32 = {};
pub global {}_BIT_P1: u32 = {};
pub global {}_BIT_P2: u32 = {};

pub global {}_Q_MOD_T_MOD_P: Field = {};
pub global {}_K0IS: [Field; L] = [{}];
pub global {}_PK_BOUNDS: [Field; L] = [{}];
pub global {}_E0_BOUND: Field = {};
pub global {}_E1_BOUND: Field = {};
pub global {}_U_BOUND: Field = {};
pub global {}_K1_LOW_BOUND: Field = {};
pub global {}_K1_UP_BOUND: Field = {};
pub global {}_R1_LOW_BOUNDS: [Field; L] = [{}];
pub global {}_R1_UP_BOUNDS: [Field; L] = [{}];
pub global {}_R2_BOUNDS: [Field; L] = [{}];
pub global {}_P1_BOUNDS: [Field; L] = [{}];
pub global {}_P2_BOUNDS: [Field; L] = [{}];

pub global {}_CONFIGS: UserDataEncryptionConfigs<N, L> = UserDataEncryptionConfigs::new(
{}_Q_MOD_T_MOD_P,
QIS,
{}_K0IS,
{}_PK_BOUNDS,
{}_E0_BOUND,
{}_E1_BOUND,
{}_U_BOUND,
{}_R1_LOW_BOUNDS,
{}_R1_UP_BOUNDS,
{}_R2_BOUNDS,
{}_P1_BOUNDS,
{}_P2_BOUNDS,
{}_K1_LOW_BOUND,
{}_K1_UP_BOUND
);
"#,
        preset.metadata().degree,     // N
        preset.metadata().num_moduli, // L
        qis_str,                      // QIS array
        prefix,
        configs.bits.pk_bit, // BIT_PK
        prefix,
        configs.bits.ct_bit, // BIT_CT
        prefix,
        configs.bits.u_bit, // BIT_U
        prefix,
        configs.bits.e0_bit, // BIT_E0
        prefix,
        configs.bits.e1_bit, // BIT_E1
        prefix,
        configs.bits.k_bit, // BIT_K
        prefix,
        configs.bits.r1_bit, // BIT_R1
        prefix,
        configs.bits.r2_bit, // BIT_R2
        prefix,
        configs.bits.p1_bit, // BIT_P1
        prefix,
        configs.bits.p2_bit, // BIT_P2
        prefix,
        configs.q_mod_t_mod_p, // Q_MOD_T_MOD_P
        prefix,
        k0is_str, // K0IS array
        prefix,
        pk_bounds_str, // PK_BOUNDS array
        prefix,
        configs.bounds.e0_bound, // E0_BOUND
        prefix,
        configs.bounds.e1_bound, // E1_BOUND
        prefix,
        configs.bounds.u_bound, // U_BOUND
        prefix,
        configs.bounds.k1_low_bound, // K1_LOW_BOUND
        prefix,
        configs.bounds.k1_up_bound, // K1_UP_BOUND
        prefix,
        r1_low_bounds_str, // R1_LOW_BOUNDS array
        prefix,
        r1_up_bounds_str, // R1_UP_BOUNDS array
        prefix,
        r2_bounds_str, // R2_BOUNDS array
        prefix,
        p1_bounds_str, // P1_BOUNDS array
        prefix,
        p2_bounds_str, // P2_BOUNDS array
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
        prefix,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuits::computation::Computation;
    use crate::codegen::write_artifacts;
    use crate::threshold::user_data_encryption::computation::{Bits, Bounds};
    use crate::threshold::user_data_encryption::sample::UserDataEncryptionSample;

    use e3_fhe_params::DEFAULT_BFV_PRESET;
    use tempfile::TempDir;

    #[test]
    fn test_toml_generation_and_structure() {
        let sample = UserDataEncryptionSample::generate(DEFAULT_BFV_PRESET);
        let artifacts = UserDataEncryptionCircuit
            .codegen(
                DEFAULT_BFV_PRESET,
                &UserDataEncryptionCircuitInput {
                    public_key: sample.public_key,
                    plaintext: sample.plaintext,
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
        let bounds = Bounds::compute(DEFAULT_BFV_PRESET, &()).unwrap();
        let bits = Bits::compute(DEFAULT_BFV_PRESET, &bounds).unwrap();

        assert!(configs_content
            .contains(format!("N: u32 = {}", DEFAULT_BFV_PRESET.metadata().degree).as_str()));
        assert!(configs_content
            .contains(format!("L: u32 = {}", DEFAULT_BFV_PRESET.metadata().num_moduli).as_str()));
        assert!(configs_content.contains(
            format!(
                "{}_BIT_PK: u32 = {}",
                <UserDataEncryptionCircuit as Circuit>::PREFIX,
                bits.pk_bit
            )
            .as_str()
        ));
    }
}
