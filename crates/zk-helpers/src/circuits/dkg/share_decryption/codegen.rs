// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Code generation for the share-decryption BFV circuit: Prover.toml and configs.nr.

use crate::circuits::computation::CircuitComputation;
use crate::circuits::dkg::share_decryption::Configs;
use crate::circuits::dkg::share_decryption::ShareDecryptionCircuit;
use crate::circuits::dkg::share_decryption::ShareDecryptionCircuitInput;
use crate::circuits::dkg::share_decryption::ShareDecryptionOutput;
use crate::circuits::dkg::share_decryption::Witness;
use crate::circuits::{Artifacts, CircuitCodegen, CircuitsErrors, CodegenToml};
use crate::codegen::CodegenConfigs;
use crate::computation::Computation;
use crate::registry::Circuit;
use e3_fhe_params::BfvPreset;

/// Implementation of [`CircuitCodegen`] for [`ShareDecryptionCircuit`].
impl CircuitCodegen for ShareDecryptionCircuit {
    type Preset = BfvPreset;
    type Input = ShareDecryptionCircuitInput;
    type Error = CircuitsErrors;

    fn codegen(&self, preset: Self::Preset, input: &Self::Input) -> Result<Artifacts, Self::Error> {
        let ShareDecryptionOutput { witness, .. } = ShareDecryptionCircuit::compute(preset, input)?;

        let toml = generate_toml(&witness)?;
        let configs = Configs::compute(preset, input)?;
        let configs_str = generate_configs(preset, &configs);

        Ok(Artifacts {
            toml,
            configs: configs_str,
        })
    }
}

/// Serializes the witness to TOML string for the Noir prover (Prover.toml).
pub fn generate_toml(witness: &Witness) -> Result<CodegenToml, CircuitsErrors> {
    let json = witness
        .to_json()
        .map_err(|e| CircuitsErrors::SerdeJson(e))?;

    Ok(toml::to_string(&json)?)
}

/// Builds the configs.nr string (N, L, bit parameters, and ShareDecryptionConfigs) for the Noir prover.
pub fn generate_configs(preset: BfvPreset, configs: &Configs) -> CodegenConfigs {
    let prefix = <ShareDecryptionCircuit as Circuit>::PREFIX;

    format!(
        r#"pub global N: u32 = {};
pub global L: u32 = {};

/************************************
-------------------------------------
share_decryption_sk (CIRCUIT 4a - BFV DECRYPTION SK)
share_decryption_e_sm (CIRCUIT 4b - BFV DECRYPTION E_SM)
-------------------------------------
************************************/

pub global {}_BIT_MSG: u32 = {};
"#,
        preset.dkg_counterpart().unwrap().metadata().degree,
        preset.dkg_counterpart().unwrap().metadata().num_moduli,
        prefix,
        configs.bits.msg_bit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::circuits::dkg::share_decryption::{Configs, ShareDecryptionCircuitInput};
    use crate::computation::{Computation, DkgInputType};
    use crate::Circuit;
    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_toml_generation_and_structure() {
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SecretKey,
        );

        let artifacts = ShareDecryptionCircuit
            .codegen(BfvPreset::InsecureThreshold512, &sample)
            .unwrap();

        let parsed: toml::Value = artifacts.toml.parse().unwrap();
        assert!(parsed.get("expected_commitments").is_some());
        assert!(parsed.get("decrypted_shares").is_some());
    }

    #[test]
    fn test_configs_generation_contains_expected() {
        let sample = ShareDecryptionCircuitInput::generate_sample(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            DkgInputType::SecretKey,
        );

        let artifacts = ShareDecryptionCircuit
            .codegen(BfvPreset::InsecureThreshold512, &sample)
            .unwrap();

        let configs = Configs::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();
        let prefix = <ShareDecryptionCircuit as Circuit>::PREFIX;
        assert!(artifacts
            .configs
            .contains(format!("{}_BIT_MSG: u32 = {}", prefix, configs.bits.msg_bit).as_str()));
    }
}
