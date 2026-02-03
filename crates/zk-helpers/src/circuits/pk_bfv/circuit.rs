// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::computation::DkgInputType;
use crate::registry::Circuit;
use e3_fhe_params::ParameterType;
use fhe::bfv::PublicKey;

#[derive(Debug)]
pub struct PkBfvCircuit;

impl Circuit for PkBfvCircuit {
    const NAME: &'static str = "pk-bfv";
    const PREFIX: &'static str = "PK_BFV";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
}

pub struct PkBfvCircuitInput {
    /// BFV public key.
    pub public_key: PublicKey,
}
