// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML serialization error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("BFV error: {0}")]
    Fhe(#[from] fhe::Error),
    #[error("ZK helper error: {0}")]
    ZkHelpers(#[from] e3_zk_helpers::utils::ZkHelpersUtilsError),
    #[error("Unexpected error: {0}")]
    Other(String),
}
