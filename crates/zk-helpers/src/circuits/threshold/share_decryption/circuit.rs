// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::computation::DkgInputType;
use crate::registry::Circuit;
use e3_fhe_params::ParameterType;
use e3_polynomial::CrtPolynomial;
use fhe::bfv::{Ciphertext, PublicKey};

#[derive(Debug)]
pub struct ShareDecryptionCircuit;

impl Circuit for ShareDecryptionCircuit {
    const NAME: &'static str = "share-decryption";
    const PREFIX: &'static str = "SHARE_DECRYPTION";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::THRESHOLD;
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
}

pub struct ShareDecryptionCircuitInput {
    pub ciphertext: Ciphertext,
    pub public_key: PublicKey,
    pub s: CrtPolynomial,
    pub e: CrtPolynomial,
    pub d_share: CrtPolynomial,
}
