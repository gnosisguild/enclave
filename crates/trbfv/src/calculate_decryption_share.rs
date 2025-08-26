use crate::{ArcBytes, TrBFVConfig};
/// This module defines event payloads that will generate a decryption share for the given ciphertext for this node
use e3_crypto::{Cipher, SensitiveBytes};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// Ciphertext to decrypt
    pub ciphertext: ArcBytes,
    /// A single summed polynomial for this nodes secret key.
    pub sk_poly_sum: SensitiveBytes,
    /// A vector of summed polynomials for this parties smudging noise
    pub es_poly_sum: Vec<SensitiveBytes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// The decryption share for the given ciphertext
    pub d_share_poly: Vec<ArcBytes>,
}

pub async fn calculate_decryption_share(cipher: &Cipher, req: Request) -> Response {
    todo!()
}
