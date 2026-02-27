// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{E3id, Proof};
use actix::Message;
use derivative::Derivative;
use e3_utils::utility_types::ArcBytes;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Exchange #3: Each honest node shares its aggregated trBFV partial key shares
/// with all other honest nodes, together with C4 proofs of correct BFV decryption.
#[derive(Message, Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
#[derivative(Debug)]
pub struct DecryptionKeyShared {
    pub e3_id: E3id,
    /// The sender's party_id.
    pub party_id: u64,
    /// The sender's node address.
    pub node: String,
    /// Lagrange-interpolated aggregated SK polynomial (serialized).
    #[derivative(Debug(format_with = "e3_utils::formatters::hexf"))]
    pub sk_poly_sum: ArcBytes,
    /// Lagrange-interpolated aggregated E_SM polynomials (serialized), one per smudging noise.
    pub es_poly_sum: Vec<ArcBytes>,
    /// C4a proof (SecretKey decryption).
    pub c4a_proof: Proof,
    /// C4b proofs (SmudgingNoise decryption), one per smudging noise index.
    pub c4b_proofs: Vec<Proof>,
    /// Whether this was received from the network.
    pub external: bool,
}

impl Display for DecryptionKeyShared {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DecryptionKeyShared {{ e3_id: {}, party_id: {} }}",
            self.e3_id, self.party_id
        )
    }
}
