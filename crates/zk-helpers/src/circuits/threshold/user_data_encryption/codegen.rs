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
use crate::CircuitCodegen;
use crate::CircuitsErrors;
use crate::{Artifacts, Toml};

use fhe::bfv::BfvParameters;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;

/// Implementation of [`CircuitCodegen`] for [`UserDataEncryptionCircuit`].
impl CircuitCodegen for UserDataEncryptionCircuit {
    type Params = Arc<BfvParameters>;
    type Input = UserDataEncryptionCircuitInput;
    type Error = CircuitsErrors;

    fn codegen(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<Artifacts, Self::Error> {
        let witness = Witness::compute(params, input)?;
        let configs = Configs::compute(params, &())?;

        let toml = generate_toml(witness)?;
        let configs = generate_configs(&params, &configs);

        Ok(Artifacts { toml, configs })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlJson {
    pub pk0is: Vec<serde_json::Value>,
    pub pk1is: Vec<serde_json::Value>,
    pub ct0is: Vec<serde_json::Value>,
    pub ct1is: Vec<serde_json::Value>,
    pub u: Vec<serde_json::Value>,
    pub e0: Vec<serde_json::Value>,
    pub e1: Vec<serde_json::Value>,
    pub k1: Vec<serde_json::Value>,
    pub r1is: Vec<serde_json::Value>,
    pub r2is: Vec<serde_json::Value>,
    pub p1is: Vec<serde_json::Value>,
    pub p2is: Vec<serde_json::Value>,
}

pub fn generate_toml(witness: Witness) -> Result<Toml, CircuitsErrors> {
    let pk0is = crt_polynomial_to_toml_json(&witness.pk0is);
    let pk1is = crt_polynomial_to_toml_json(&witness.pk1is);
    let ct0is = crt_polynomial_to_toml_json(&witness.ct0is);
    let ct1is = crt_polynomial_to_toml_json(&witness.ct1is);
    let u = polynomial_to_toml_json(&witness.u);
    let e0 = polynomial_to_toml_json(&witness.e0);
    let e1 = polynomial_to_toml_json(&witness.e1);
    let k1 = polynomial_to_toml_json(&witness.k1);
    let r1is = crt_polynomial_to_toml_json(&witness.r1is);
    let r2is = crt_polynomial_to_toml_json(&witness.r2is);
    let p1is = crt_polynomial_to_toml_json(&witness.p1is);
    let p2is = crt_polynomial_to_toml_json(&witness.p2is);

    let toml_json = TomlJson {
        pk0is,
        pk1is,
        ct0is,
        ct1is,
        u,
        e0,
        e1,
        k1,
        r1is,
        r2is,
        p1is,
        p2is,
    };

    Ok(toml::to_string(&toml_json)?)
}

pub fn generate_configs(params: &Arc<BfvParameters>, configs: &Configs) -> String {
    let qis_str = join_display(&configs.moduli, ", ");
    let k0is_str = join_display(&configs.k0is, ", ");
    let pk_bounds_str = join_display(&configs.bounds.pk_bounds, ", ");
    let r1_low_bounds_str = join_display(&configs.bounds.r1_low_bounds, ", ");
    let r1_up_bounds_str = join_display(&configs.bounds.r1_up_bounds, ", ");
    let r2_bounds_str = join_display(&configs.bounds.r2_bounds, ", ");
    let p1_bounds_str = join_display(&configs.bounds.p1_bounds, ", ");
    let p2_bounds_str = join_display(&configs.bounds.p2_bounds, ", ");

    format!(
        r#"use crate::core::greco::Configs as GrecoConfigs;

// Global configs for Greco circuit
pub global N: u32 = {};
pub global L: u32 = {};
pub global QIS: [Field; L] = [{}];

/************************************
-------------------------------------
greco (USED FOR ENCRYPTION TRBFV x PVSS)
-------------------------------------
************************************/

// greco - bit parameters
pub global GRECO_BIT_PK: u32 = {};
pub global GRECO_BIT_CT: u32 = {};
pub global GRECO_BIT_U: u32 = {};
pub global GRECO_BIT_E0: u32 = {};
pub global GRECO_BIT_E1: u32 = {};
pub global GRECO_BIT_K: u32 = {};
pub global GRECO_BIT_R1: u32 = {};
pub global GRECO_BIT_R2: u32 = {};
pub global GRECO_BIT_P1: u32 = {};
pub global GRECO_BIT_P2: u32 = {};

// greco - bounds
pub global GRECO_Q_MOD_T: Field = {};
pub global GRECO_Q_MOD_T_MOD_P: Field = {};
pub global GRECO_K0IS: [Field; L] = [{}];
pub global GRECO_PK_BOUNDS: [Field; L] = [{}];
pub global GRECO_E0_BOUND: Field = {};
pub global GRECO_E1_BOUND: Field = {};
pub global GRECO_U_BOUND: Field = {};
pub global GRECO_K1_LOW_BOUND: Field = {};
pub global GRECO_K1_UP_BOUND: Field = {};
pub global GRECO_R1_LOW_BOUNDS: [Field; L] = [{}];
pub global GRECO_R1_UP_BOUNDS: [Field; L] = [{}];
pub global GRECO_R2_BOUNDS: [Field; L] = [{}];
pub global GRECO_P1_BOUNDS: [Field; L] = [{}];
pub global GRECO_P2_BOUNDS: [Field; L] = [{}];

// greco - configs
pub global GRECO_CONFIGS: GrecoConfigs<N, L> = GrecoConfigs::new(
GRECO_Q_MOD_T,
GRECO_Q_MOD_T_MOD_P,
QIS,
GRECO_K0IS,
GRECO_PK_BOUNDS,
GRECO_E0_BOUND,
GRECO_E1_BOUND,
GRECO_U_BOUND,
GRECO_R1_LOW_BOUNDS,
GRECO_R1_UP_BOUNDS,
GRECO_R2_BOUNDS,
GRECO_P1_BOUNDS,
GRECO_P2_BOUNDS,
GRECO_K1_LOW_BOUND,
GRECO_K1_UP_BOUND
);
"#,
        params.degree(),             // N
        params.moduli().len(),       // L
        qis_str,                     // QIS array
        configs.bits.pk_bit,         // BIT_PK
        configs.bits.ct_bit,         // BIT_CT
        configs.bits.u_bit,          // BIT_U
        configs.bits.e0_bit,         // BIT_E0
        configs.bits.e1_bit,         // BIT_E1
        configs.bits.k_bit,          // BIT_K
        configs.bits.r1_bit,         // BIT_R1
        configs.bits.r2_bit,         // BIT_R2
        configs.bits.p1_bit,         // BIT_P1
        configs.bits.p2_bit,         // BIT_P2
        configs.q_mod_t,             // Q_MOD_T
        configs.q_mod_t_mod_p,       // Q_MOD_T_MOD_P
        k0is_str,                    // K0IS array
        pk_bounds_str,               // PK_BOUNDS array
        configs.bounds.e0_bound,     // E0_BOUND
        configs.bounds.e1_bound,     // E1_BOUND
        configs.bounds.u_bound,      // U_BOUND
        configs.bounds.k1_low_bound, // K1_LOW_BOUND
        configs.bounds.k1_up_bound,  // K1_UP_BOUND
        r1_low_bounds_str,           // R1_LOW_BOUNDS array
        r1_up_bounds_str,            // R1_UP_BOUNDS array
        r2_bounds_str,               // R2_BOUNDS array
        p1_bounds_str,               // P1_BOUNDS array
        p2_bounds_str,               // P2_BOUNDS array
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuits::computation::Computation;
    use crate::codegen::write_artifacts;
    use crate::threshold::user_data_encryption::computation::{Bits, Bounds};
    use crate::threshold::user_data_encryption::sample::Sample;

    use e3_fhe_params::BfvParamSet;
    use e3_fhe_params::DEFAULT_BFV_PRESET;
    use tempfile::TempDir;

    #[test]
    fn test_toml_generation_and_structure() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let sample = Sample::generate(&params);
        let artifacts = UserDataEncryptionCircuit
            .codegen(
                &params,
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
        let bounds = Bounds::compute(&params, &()).unwrap();
        let bits = Bits::compute(&params, &bounds).unwrap();

        assert!(configs_content.contains(format!("N: u32 = {}", params.degree()).as_str()));
        assert!(configs_content.contains(format!("L: u32 = {}", params.moduli().len()).as_str()));
        assert!(configs_content.contains(format!("GRECO_BIT_PK: u32 = {}", bits.pk_bit).as_str()));
    }
}
