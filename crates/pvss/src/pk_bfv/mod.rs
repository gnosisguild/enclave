// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod codegen;
pub mod computation;

use crate::traits::Circuit;
use crate::types::DkgInputType;
use e3_fhe_params::ParameterType;

pub struct PkBfvCircuit;

impl Circuit for PkBfvCircuit {
    const NAME: &'static str = "pk-bfv";
    const PREFIX: &'static str = "PK_BFV";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
    const N_PROOFS: usize = 1;
    const N_PUBLIC_INPUTS: usize = 1;
}
