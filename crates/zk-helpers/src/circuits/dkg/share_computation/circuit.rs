// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::computation::DkgInputType;
use crate::registry::Circuit;
use e3_fhe_params::ParameterType;
use e3_polynomial::CrtPolynomial;
use ndarray::Array2;
use num_bigint::BigInt;

#[derive(Debug)]
pub struct ShareComputationCircuit;

impl Circuit for ShareComputationCircuit {
    const NAME: &'static str = "share-computation";
    const PREFIX: &'static str = "SHARE_COMPUTATION";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    const DKG_INPUT_TYPE: Option<DkgInputType> = match DkgInputType::SecretKey {
        DkgInputType::SecretKey => Some(DkgInputType::SecretKey),
        DkgInputType::SmudgingNoise => Some(DkgInputType::SmudgingNoise),
    };
}

pub struct ShareComputationCircuitInput {
    /// Which secret type this input is for (determines which branch to use in witness).
    pub dkg_input_type: DkgInputType,
    pub secret: CrtPolynomial,
    pub secret_sss: Vec<Array2<BigInt>>,
    pub parity_matrix: Vec<Vec<Vec<BigInt>>>,
    pub n_parties: u32,
    pub threshold: u32,
}
