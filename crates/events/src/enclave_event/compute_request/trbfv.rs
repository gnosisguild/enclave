// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};

use super::bytes::Bytes;

/// TrBFV modules defining the API for multithreaded compute
/// Each module defines the event payloads that make up a compute request
/// Each compute request should live independently and be self contained

/// Input format for TrBFVRequest
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVRequest {
    GenEsiSss(gen_esi_sss::Request),
    GenPkShareAndSkSss(gen_pk_share_and_sk_sss::Request),
    GenDecryptionKey(gen_decryption_key::Request),
    GenDecryptionShare(gen_decryption_share::Request),
    ThresholdDecrypt(threshold_decrypt::Request),
}

/// Result format for TrBFVResponse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVResponse {
    GenEsiSss(gen_esi_sss::Response),
    GenPkShareAndSkSss(gen_pk_share_and_sk_sss::Response),
    GenDecryptionKey(gen_decryption_key::Response),
    GenDecryptionShare(gen_decryption_share::Response),
    ThresholdDecrypt(threshold_decrypt::Response),
}

/// Convenience struct for holding threshold BFV configuration parameters
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrBFVConfig {
    /// BFV Params
    params: Bytes,
    /// Number of ciphernodes
    num_parties: u64,
    /// Threshold required
    threshold: u64,
}

impl TrBFVConfig {
    /// Constructor for the TrBFVConfig
    pub fn new(params: Bytes, num_parties: u64, threshold: u64) -> Self {
        Self {
            params,
            num_parties,
            threshold,
        }
    }

    pub fn params(&self) -> Bytes {
        self.params.clone() // NOTE: It might make sense to deserialize
                            // stright to BfvParameters here
                            // but leaving like this for now
    }

    pub fn num_parties(&self) -> u64 {
        self.num_parties
    }

    pub fn threshold(&self) -> u64 {
        self.threshold
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVError {
    // Add errors here as required
}

// NOTE: All size values use u64 instead of usize to maintain a stable
// protocol that works across different architectures. Convert these
// u64 values to usize when entering the library's internal APIs.

pub mod gen_esi_sss {
    /// This module defines event payloads that will generate esi smudging noise shamir shares to be shared with other members of the committee.
    /// This has been separated from the general setup in order to be able to take advantage of parallelism
    use crate::bytes::Bytes;
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    use super::TrBFVConfig;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        trbfv_config: TrBFVConfig,
        /// Error Size extracted from the E3 Program Parameters
        error_size: Bytes,
        /// Smudging noise per ciphertext
        esi_per_ct: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// The smudging noise shares
        esi_sss: Vec<SensitiveBytes>,
    }
}

pub mod gen_pk_share_and_sk_sss {
    /// This module defines event payloads that will generate the public key share as well as the sk shamir secret shares to be distributed to other members of the committee.
    /// This has been separated from the esi setup in order to be able to take advantage of parallelism
    use crate::bytes::Bytes;
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    use super::TrBFVConfig;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        trbfv_config: TrBFVConfig,
        /// Crp
        crp: Bytes,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// PublicKey share for this node
        pk_share: Bytes,
        /// SecretKey Shamir Shares for other parties
        sk_sss: Vec<SensitiveBytes>,
    }
}

pub mod gen_decryption_key {
    /// This module defines event payloads that will generate the decryption key material to create a decryption share
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    use super::TrBFVConfig;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        trbfv_config: TrBFVConfig,
        /// All collected secret key shamir shares
        sk_sss_collected: Vec<SensitiveBytes>,
        /// All collected smudging noise shamir shares
        esi_sss_collected: Vec<SensitiveBytes>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// A single summed polynomial for this nodes secret key.
        sk_poly_sum: SensitiveBytes,
        /// A single summed polynomial for this partys smudging noise
        es_poly_sum: SensitiveBytes,
    }
}

pub mod gen_decryption_share {
    /// This module defines event payloads that will generate a decryption share for the given ciphertext for this node
    use crate::bytes::Bytes;
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    use super::TrBFVConfig;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        trbfv_config: TrBFVConfig,
        /// Ciphertext to decrypt
        ciphertext: Bytes,
        /// A single summed polynomial for this nodes secret key.
        sk_poly_sum: SensitiveBytes,
        /// A single summed polynomial for this partys smudging noise
        es_poly_sum: SensitiveBytes,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// The decryption share for the given ciphertext
        d_share_poly: Bytes,
    }
}

pub mod threshold_decrypt {
    /// This module defines event payloads that will decrypt a ciphertext with a threshold quorum of decryption shares
    use crate::bytes::Bytes;
    use serde::{Deserialize, Serialize};

    use super::TrBFVConfig;

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        trbfv_config: TrBFVConfig,
        /// Ciphertext to decrypt
        ciphertext: Bytes,
        /// All decryption shares from a threshold quorum of nodes polys
        d_share_polys: Vec<(u64, Bytes)>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// The resultant plaintext
        plaintext: Bytes,
    }
}
