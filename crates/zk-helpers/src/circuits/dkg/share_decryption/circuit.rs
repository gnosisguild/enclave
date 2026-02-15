// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Circuit definition and input type for the share-decryption ZK circuit (CIRCUIT 4a/4b).

use crate::computation::DkgInputType;
use crate::registry::Circuit;
use e3_fhe_params::ParameterType;
use fhe::bfv::Ciphertext;
use fhe::bfv::SecretKey;

/// Share-decryption circuit: proves correct decryption of H honest parties' ciphertexts under the DKG secret key.
#[derive(Debug)]
pub struct ShareDecryptionCircuit;

impl Circuit for ShareDecryptionCircuit {
    const NAME: &'static str = "share-decryption";
    const PREFIX: &'static str = "SHARE_DECRYPTION";
    const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
    /// None: circuit accepts runtime-varying input type (SecretKey or SmudgingNoise).
    const DKG_INPUT_TYPE: Option<DkgInputType> = None;
}

/// Data for the share-decryption circuit: secret key and honest parties' ciphertexts.
pub struct ShareDecryptionCircuitData {
    /// DKG secret key used to decrypt (private input).
    pub secret_key: SecretKey,
    /// Ciphertexts from H honest parties: [party_idx][mod_idx] (one ciphertext per party per TRBFV modulus).
    pub honest_ciphertexts: Vec<Vec<Ciphertext>>,
    /// Which input type (SecretKey or SmudgingNoise) to resolve circuit path.
    pub dkg_input_type: DkgInputType,
}
