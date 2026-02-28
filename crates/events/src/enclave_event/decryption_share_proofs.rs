// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Events for C4 proof generation and signing flow.
//!
//! `DecryptionShareProofsPending` is published by [`ThresholdKeyshare`] when it
//! has computed the decryption data and needs C4 proofs generated and signed.
//!
//! `DecryptionShareProofsSigned` is published by [`ProofRequestActor`] after it
//! has generated C4 proofs, signed them, and is returning them to
//! [`ThresholdKeyshare`] for Exchange #3 publication.

use crate::{DkgShareDecryptionProofRequest, E3id, SignedProofPayload};
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};

/// ThresholdKeyshare → ProofRequestActor: generate and sign C4 proofs.
///
/// Carries both the proof generation inputs (sk_request, esm_requests)
/// and the protocol data (sk_poly_sum, es_poly_sum, node) so that
/// ProofRequestActor can pass them back in [`DecryptionShareProofsSigned`].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecryptionShareProofsPending {
    pub e3_id: E3id,
    pub party_id: u64,
    pub node: String,
    /// Decrypted SK polynomial sum (for Exchange #3).
    pub sk_poly_sum: ArcBytes,
    /// Decrypted ES polynomial sums (for Exchange #3).
    pub es_poly_sum: Vec<ArcBytes>,
    /// C4a proof request (SecretKey decryption).
    pub sk_request: DkgShareDecryptionProofRequest,
    /// C4b proof requests (SmudgingNoise decryption), one per ESI index.
    pub esm_requests: Vec<DkgShareDecryptionProofRequest>,
}

/// ProofRequestActor → ThresholdKeyshare: signed C4 proofs ready.
///
/// ThresholdKeyshare combines these with state data to publish
/// `DecryptionKeyShared` (Exchange #3).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecryptionShareProofsSigned {
    pub e3_id: E3id,
    pub party_id: u64,
    pub node: String,
    pub sk_poly_sum: ArcBytes,
    pub es_poly_sum: Vec<ArcBytes>,
    pub signed_sk_decryption_proof: SignedProofPayload,
    pub signed_esm_decryption_proofs: Vec<SignedProofPayload>,
}
