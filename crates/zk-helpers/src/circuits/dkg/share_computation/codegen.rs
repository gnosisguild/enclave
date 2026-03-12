// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Code generation for the share-computation helper scripts: Prover.toml and configs.nr.

use crate::circuits::computation::{CircuitComputation, Computation};
use crate::circuits::dkg::share_computation::{
    utils::parity_matrix_constant_string, Bits, Bounds, ChunkInputs, Configs, Inputs,
    ShareComputationBaseCircuit, ShareComputationChunkCircuit, ShareComputationChunkCircuitData,
    ShareComputationCircuit, ShareComputationCircuitData, ShareComputationOutput,
};
use crate::circuits::{Artifacts, CircuitCodegen, CircuitsErrors, CodegenToml};
use crate::codegen::CodegenConfigs;
use crate::registry::Circuit;
use e3_fhe_params::{build_pair_for_preset, BfvPreset};

/// Implementation of [`CircuitCodegen`] for the shared share-computation input builder.
impl CircuitCodegen for ShareComputationCircuit {
    type Preset = BfvPreset;
    type Data = ShareComputationCircuitData;
    type Error = CircuitsErrors;

    fn codegen(&self, preset: Self::Preset, data: &Self::Data) -> Result<Artifacts, Self::Error> {
        let ShareComputationOutput { inputs, bits, .. } =
            ShareComputationCircuit::compute(preset, data)?;
        let configs = Configs::compute(preset, data)?;

        build_base_artifacts(preset, data, inputs, bits, &configs)
    }
}

impl CircuitCodegen for ShareComputationBaseCircuit {
    type Preset = BfvPreset;
    type Data = ShareComputationCircuitData;
    type Error = CircuitsErrors;

    fn codegen(&self, preset: Self::Preset, data: &Self::Data) -> Result<Artifacts, Self::Error> {
        let ShareComputationOutput { inputs, bits, .. } =
            ShareComputationCircuit::compute(preset, data)?;
        let configs = Configs::compute(preset, data)?;

        build_base_artifacts(preset, data, inputs, bits, &configs)
    }
}

impl CircuitCodegen for ShareComputationChunkCircuit {
    type Preset = BfvPreset;
    type Data = ShareComputationChunkCircuitData;
    type Error = CircuitsErrors;

    fn codegen(&self, preset: Self::Preset, data: &Self::Data) -> Result<Artifacts, Self::Error> {
        let inputs = ChunkInputs::compute(preset, data)?;
        let bounds = Bounds::compute(preset, &data.share_data)?;
        let bits = Bits::compute(preset, &bounds)?;
        let configs = Configs::compute(preset, &data.share_data)?;

        Ok(Artifacts {
            toml: generate_chunk_toml(&inputs)?,
        configs: generate_configs(
            preset,
            &bits,
            data.share_data.n_parties as usize,
            data.share_data.threshold as usize,
            configs.chunk_size,
            configs.chunks_per_batch,
            configs.n_batches,
        )?,
        })
    }
}

fn build_base_artifacts(
    preset: BfvPreset,
    data: &ShareComputationCircuitData,
    inputs: Inputs,
    bits: Bits,
    configs: &Configs,
) -> Result<Artifacts, CircuitsErrors> {
    Ok(Artifacts {
        toml: generate_toml(&inputs)?,
        configs: generate_configs(
            preset,
            &bits,
            data.n_parties as usize,
            data.threshold as usize,
            configs.chunk_size,
            configs.chunks_per_batch,
            configs.n_batches,
        )?,
    })
}

pub fn generate_toml(witness: &Inputs) -> Result<CodegenToml, CircuitsErrors> {
    let json = witness.to_json().map_err(CircuitsErrors::SerdeJson)?;
    Ok(toml::to_string(&json)?)
}

pub fn generate_chunk_toml(witness: &ChunkInputs) -> Result<CodegenToml, CircuitsErrors> {
    let json = witness.to_json().map_err(CircuitsErrors::SerdeJson)?;
    Ok(toml::to_string(&json)?)
}

/// Builds the configs.nr string used by the split base/chunk share-computation circuits.
pub fn generate_configs(
    preset: BfvPreset,
    bits: &Bits,
    n_parties: usize,
    threshold: usize,
    chunk_size: usize,
    chunks_per_batch: usize,
    _n_batches: usize,
) -> Result<CodegenConfigs, CircuitsErrors> {
    let (threshold_params, _) =
        build_pair_for_preset(preset).map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
    let config_name = preset.metadata().security.as_config_str();
    let parity_matrix_str = parity_matrix_constant_string(&threshold_params, n_parties, threshold)?;
    let prefix = <ShareComputationCircuit as Circuit>::PREFIX;

    Ok(format!(
        r#"use crate::core::dkg::share_computation::chunk::Configs as ShareComputationChunkConfigs;
pub use crate::configs::{config_name}::threshold::{{L as L_THRESHOLD, QIS as QIS_THRESHOLD}};

pub global N: u32 = {degree};

{parity_matrix}

/************************************
-------------------------------------
share_computation_sk (CIRCUIT 2a)
-------------------------------------
************************************/

pub global {prefix}_BIT_SHARE: u32 = {bit_share};
pub global {prefix}_SK_BIT_SECRET: u32 = {bit_sk_secret};

/************************************
-------------------------------------
share_computation_e_sm (CIRCUIT 2b)
-------------------------------------
************************************/

pub global {prefix}_E_SM_BIT_SECRET: u32 = {bit_e_sm_secret};

/************************************
-------------------------------------
share_computation_chunk (CIRCUIT 2c)
-------------------------------------
************************************/

pub global {prefix}_CHUNK_SIZE: u32 = {chunk_size};
pub global {prefix}_N_CHUNKS: u32 = N / {prefix}_CHUNK_SIZE;

pub global {prefix}_CHUNKS_PER_BATCH: u32 = {chunks_per_batch};
pub global {prefix}_N_BATCHES: u32 =
    {prefix}_N_CHUNKS / {prefix}_CHUNKS_PER_BATCH;

pub global {prefix}_CHUNK_CONFIGS: ShareComputationChunkConfigs<L_THRESHOLD> =
    ShareComputationChunkConfigs::new(QIS_THRESHOLD);
"#,
        config_name = config_name,
        degree = preset.metadata().degree,
        parity_matrix = parity_matrix_str,
        prefix = prefix,
        bit_share = bits.bit_share,
        bit_sk_secret = bits.bit_sk_secret,
        bit_e_sm_secret = bits.bit_e_sm_secret,
        chunk_size = chunk_size,
        chunks_per_batch = chunks_per_batch,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::circuits::computation::Computation;
    use crate::circuits::dkg::share_computation::{Bits, Bounds, ShareComputationChunkCircuitData};
    use crate::codegen::write_artifacts;
    use crate::computation::DkgInputType;
    use crate::Circuit;
    use e3_fhe_params::BfvPreset;
    use tempfile::TempDir;

    #[test]
    fn test_toml_generation_and_structure() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareComputationCircuitData::generate_sample(
            BfvPreset::InsecureThreshold512,
            committee,
            DkgInputType::SecretKey,
        )
        .unwrap();

        let artifacts = ShareComputationCircuit
            .codegen(BfvPreset::InsecureThreshold512, &sample)
            .unwrap();

        let parsed: toml::Value = artifacts.toml.parse().unwrap();
        let sk_secret = parsed.get("sk_secret").unwrap();
        assert!(sk_secret
            .get("coefficients")
            .and_then(|c| c.as_array())
            .is_some());
        let y = parsed.get("y").and_then(|v| v.as_array()).unwrap();
        assert!(!y.is_empty());
        assert!(parsed.get("expected_secret_commitment").is_some());

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
        assert!(content.contains("sk_secret"));
        assert!(content.contains("expected_secret_commitment"));
        assert!(content.contains("y"));

        let configs_path = temp_dir.path().join("configs.nr");
        assert!(configs_path.exists());

        let configs_content = std::fs::read_to_string(&configs_path).unwrap();
        let bounds = Bounds::compute(BfvPreset::InsecureThreshold512, &sample).unwrap();
        let bits = Bits::compute(BfvPreset::InsecureThreshold512, &bounds).unwrap();
        let prefix = <ShareComputationCircuit as Circuit>::PREFIX;

        assert!(configs_content.contains(
            format!(
                "N: u32 = {}",
                BfvPreset::InsecureThreshold512.metadata().degree
            )
            .as_str()
        ));
        assert!(configs_content
            .contains(format!("{}_BIT_SHARE: u32 = {}", prefix, bits.bit_share).as_str()));
        assert!(configs_content
            .contains(format!("{}_SK_BIT_SECRET: u32 = {}", prefix, bits.bit_sk_secret).as_str()));
        assert!(configs_content.contains(format!("{}_CHUNK_SIZE: u32 = {}", prefix, 512).as_str()));
        assert!(configs_content.contains(
            format!("{}_N_CHUNKS: u32 = N / {}_CHUNK_SIZE", prefix, prefix).as_str()
        ));
        assert!(configs_content.contains(
            format!("{}_CHUNKS_PER_BATCH: u32 = 1", prefix).as_str()
        ));
        assert!(configs_content.contains(
            format!("{}_N_BATCHES: u32 =", prefix).as_str()
        ));
        assert!(configs_content.contains(
            format!("{}_E_SM_BIT_SECRET: u32 = {}", prefix, bits.bit_e_sm_secret).as_str()
        ));
    }

    #[test]
    fn test_chunk_toml_generation_and_structure() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = ShareComputationChunkCircuitData::generate_sample(
            BfvPreset::SecureThreshold8192,
            committee,
            DkgInputType::SecretKey,
            1,
        )
        .unwrap();

        let artifacts = ShareComputationChunkCircuit
            .codegen(BfvPreset::SecureThreshold8192, &sample)
            .unwrap();

        let parsed: toml::Value = artifacts.toml.parse().unwrap();
        let y_chunk = parsed.get("y_chunk").and_then(|v| v.as_array()).unwrap();
        assert_eq!(y_chunk.len(), 512);
    }
}
