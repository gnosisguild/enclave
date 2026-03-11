// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::computation::DkgInputType;
use crate::registry::Circuit;
use crate::CircuitsErrors;
use e3_fhe_params::BfvPreset;
use e3_fhe_params::ParameterType;
use e3_parity_matrix::ParityMatrix;
use e3_polynomial::CrtPolynomial;
use ndarray::Array2;
use num_bigint::BigInt;

// todo: remove this (keep it until we update zk-prover)
#[derive(Debug)]
pub struct ShareComputationCircuit;

impl Circuit for ShareComputationCircuit {
    const NAME: &'static str = "share-computation";
    const PREFIX: &'static str = "SHARE_COMPUTATION";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::THRESHOLD;
    /// None: circuit accepts runtime-varying input type (SecretKey or SmudgingNoise via `ShareComputationCircuitInput::dkg_input_type`).
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
}

#[derive(Debug)]
pub struct ShareComputationBaseCircuit;

impl Circuit for ShareComputationBaseCircuit {
    const NAME: &'static str = "share-computation-base";
    const PREFIX: &'static str = "SHARE_COMPUTATION";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::THRESHOLD;
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
}

#[derive(Debug)]
pub struct ShareComputationChunkCircuit;

impl Circuit for ShareComputationChunkCircuit {
    const NAME: &'static str = "share-computation-chunk";
    const PREFIX: &'static str = "SHARE_COMPUTATION";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::THRESHOLD;
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
}

// todo: currently reusing this but should be renamed when we change zk-prover
#[derive(Clone)]
pub struct ShareComputationCircuitData {
    /// Which secret type this data is for (determines which branch to use in data).
    pub dkg_input_type: DkgInputType,
    pub secret: CrtPolynomial,
    pub secret_sss: Vec<Array2<BigInt>>,
    pub parity_matrix: Vec<ParityMatrix>,
    pub n_parties: u32,
    pub threshold: u32,
}

pub struct ShareComputationChunkCircuitData {
    pub share_data: ShareComputationCircuitData,
    pub chunk_idx: usize,
}

impl ShareComputationChunkCircuitData {
    pub fn generate_sample(
        preset: BfvPreset,
        committee: crate::CiphernodesCommittee,
        dkg_input_type: DkgInputType,
        chunk_idx: usize,
    ) -> Result<Self, CircuitsErrors> {
        Ok(Self {
            share_data: ShareComputationCircuitData::generate_sample(
                preset,
                committee,
                dkg_input_type,
            )?,
            chunk_idx,
        })
    }
}
