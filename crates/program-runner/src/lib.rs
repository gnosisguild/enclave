// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Serves a project's E3 program to the ciphernodes. The project owns its program and passes it in
//! from a small runner binary:
//!
//! ```ignore
//! fn main() -> anyhow::Result<()> {
//!     e3_program_runner::run(e3_user_program::fhe_processor, e3_user_program::policy())
//! }
//! ```
//!
//! `interfold program start` runs that binary with one argument: `dev` serves the unproved
//! development runner, and `openvm` serves the OpenVM proving service.

mod dev;
mod openvm;

use anyhow::{bail, Result};

pub use e3_compute_provider::{FHEProcessor, InputPolicy};

/// Serves `processor` under `policy` in the mode that the first command-line argument names.
pub fn run(processor: FHEProcessor, policy: InputPolicy) -> Result<()> {
    let runtime = || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
    };
    let mut args = std::env::args_os();
    let binary = args.next().unwrap_or_default();
    match args.next().as_deref().and_then(|mode| mode.to_str()) {
        Some("dev") => runtime()?.block_on(dev::serve(processor, policy)),
        Some("openvm") => runtime()?.block_on(openvm::serve(processor, policy)),
        _ => bail!("Usage: {} dev|openvm", binary.to_string_lossy()),
    }
}
