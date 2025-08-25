// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};

/// TrBFV modules defining the API for multithreaded compute
/// Each module defines the event payloads that make up a compute request
/// Each compute request should live independently and be self contained

/// Input format for TrBFVRequest
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVRequest {
    GenEsiSss(gen_esi_sss::Request),
    GenPkShareAndSkSss(gen_pk_share_and_sk_sss::Request),
    CalculateDecryptionKey(calculate_decryption_key::Request),
    CalculateDecryptionShare(calculate_decryption_share::Request),
    CalculateThresholdDecryption(calculate_threshold_decyption::Request),
}

/// Result format for TrBFVResponse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrBFVResponse {
    GenEsiSss(gen_esi_sss::Response),
    GenPkShareAndSkSss(gen_pk_share_and_sk_sss::Response),
    CalculateDecryptionKey(calculate_decryption_key::Response),
    CalculateDecryptionShare(calculate_decryption_share::Response),
    CalculateThresholdDecryption(calculate_threshold_decyption::Response),
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
    use e3_crypto::SensitiveBytes;
    use e3_trbfv::{ArcBytes, TrBFVConfig};
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
}

pub mod gen_pk_share_and_sk_sss {
    /// This module defines event payloads that will generate the public key share as well as the sk shamir secret shares to be distributed to other members of the committee.
    /// This has been separated from the esi setup in order to be able to take advantage of parallelism
    use e3_crypto::SensitiveBytes;
    use e3_trbfv::{ArcBytes, TrBFVConfig};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        pub trbfv_config: TrBFVConfig,
        /// Crp
        pub crp: ArcBytes,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// PublicKey share for this node
        pub pk_share: ArcBytes,
        /// SecretKey Shamir Shares for other parties
        pub sk_sss: Vec<SensitiveBytes>,
    }
}

pub mod calculate_decryption_key {
    /// This module defines event payloads that will generate the decryption key material to create a decryption share
    use e3_crypto::SensitiveBytes;
    use e3_trbfv::TrBFVConfig;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        pub trbfv_config: TrBFVConfig,
        /// All collected secret key shamir shares
        pub sk_sss_collected: Vec<SensitiveBytes>,
        /// All collected smudging noise shamir shares
        pub esi_sss_collected: Vec<SensitiveBytes>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// A single summed polynomial for this nodes secret key.
        pub sk_poly_sum: SensitiveBytes,
        /// A single summed polynomial for this partys smudging noise
        pub es_poly_sum: Vec<SensitiveBytes>,
    }
}

pub mod calculate_decryption_share {
    /// This module defines event payloads that will generate a decryption share for the given ciphertext for this node
    use e3_crypto::SensitiveBytes;
    use e3_trbfv::{ArcBytes, TrBFVConfig};
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
}
pub mod calculate_threshold_decyption {
    /// This module defines event payloads that will dcrypt a ciphertext with a threshold quorum of decryption shares
    use e3_trbfv::{ArcBytes, PartyId, TrBFVConfig};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// TrBFV configuration
        pub trbfv_config: TrBFVConfig,
        /// All decryption shares from a threshold quorum of nodes polys.
        pub d_share_polys: Vec<(PartyId, ArcBytes)>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// The resultant plaintext
        pub plaintext: ArcBytes,
    }
}
