// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::computation::DkgInputType;
use crate::registry::Circuit;
use e3_fhe_params::ParameterType;
use ndarray::Array2;
use num_bigint::BigInt;

#[derive(Debug)]
pub struct ShareComputationSkCircuit;

impl Circuit for ShareComputationSkCircuit {
    const NAME: &'static str = "share_computation_sk";
    const PREFIX: &'static str = "SHARE_COMPUTATION_SK";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    const DKG_INPUT_TYPE: Option<DkgInputType> = Some(DkgInputType::SecretKey);
}

#[derive(Debug)]
pub struct ShareComputationEsmCircuit;

impl Circuit for ShareComputationEsmCircuit {
    const NAME: &'static str = "share_computation_e_sm";
    const PREFIX: &'static str = "SHARE_COMPUTATION_E_SM";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    const DKG_INPUT_TYPE: Option<DkgInputType> = Some(DkgInputType::SmudgingNoise);
}

pub struct ShareComputationCircuitInput {
    pub secret_coefficients: Vec<BigInt>,
    pub secret_sss: Vec<Array2<BigInt>>,
    pub parity_matrix: Vec<Vec<Vec<BigInt>>>,
    pub n_parties: u32,
    pub threshold: u32,
}
