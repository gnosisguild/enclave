// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Share-decryption circuit: proves correct decryption of honest parties' ciphertexts under the DKG secret key (CIRCUIT 4a/4b).

pub mod circuit;
pub mod codegen;
pub mod computation;
pub mod sample;
pub use circuit::{ShareDecryptionCircuit, ShareDecryptionCircuitInput};
pub use computation::{Bits, Bounds, Configs, ShareDecryptionOutput, Witness};
pub use sample::{prepare_share_decryption_sample_for_test, ShareDecryptionSample};
