// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod config;
mod error;
mod prover;
mod setup;

pub use config::{NoirConfig, VersionInfo};
pub use error::NoirProverError;
pub use prover::NoirProver;
pub use setup::{NoirSetup, SetupStatus};
