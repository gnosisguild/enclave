// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// TrBFV modules defining the API for multithreaded compute
/// Each module defines the events that make up a compute request
/// Each compute request should live independently and be self contained

/// This method will generate esi smudging noise shamir shares to be shared with other members of the committee.
/// This has been separated from the general setup in order to be able to take advantage of parallelism
pub mod gen_esi_sss {
    use crate::bytes::Bytes;
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// BFV Params
        params: Bytes,
        /// Max number of ciphertexts
        max_num_ciphertexts: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// The smudging noise shares
        esi_sss: Vec<SensitiveBytes>,
    }
}

/// This method will generate the public key share as well as the sk shamir secret shares to be distributed to other members of the committee
/// This has been separated from the esi setup in order to be able to take advantage of parallelism
pub mod gen_pk_share_and_sk_sss {
    use crate::bytes::Bytes;
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// BFV Params
        params: Bytes,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// PublicKey share for this node
        pk_share: Bytes,
        /// SecretKey Shamir Shares for other parties
        sk_sss: Vec<SensitiveBytes>,
    }
}

/// This method will generate the decryption key material to create a decryption share
pub mod gen_decryption_key {
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
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

/// This method will generate a decryption share for the given ciphertext for this node
pub mod gen_decryption_share {
    use crate::bytes::Bytes;
    use e3_crypto::SensitiveBytes;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// BFV Params
        params: Bytes,
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

/// This method will decrypt a ciphertext with a threshold quorum of decryption shares
pub mod threshold_decrypt {
    use crate::bytes::Bytes;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Request {
        /// BFV Params
        params: Bytes,
        /// Ciphertext to decrypt
        ciphertext: Bytes,
        /// All decryption shares from a threshold quorum of nodes
        d_share_polys: Vec<Bytes>,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Response {
        /// The resultant plaintext
        plaintext: Bytes,
    }
}
