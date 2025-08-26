use crate::{ArcBytes, TrBFVConfig};
use e3_crypto::{Cipher, SensitiveBytes};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Request {
    /// TrBFV configuration
    pub trbfv_config: TrBFVConfig,
    /// Error Size extracted from the E3 Program Parameters
    pub error_size: ArcBytes,
    /// Smudging noise per ciphertext
    pub esi_per_ct: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Response {
    /// The smudging noise shares
    pub esi_sss: Vec<SensitiveBytes>,
}

pub async fn gen_esi_sss(cipher: &Cipher, req: Request) -> Response {
    todo!()
}
