// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fs, path::Path};

use anyhow::Result;
use tempfile::TempDir;

/// Get the tempdir within ./target/tmp. This is important since some virtual environments such as nix
/// won't necessarily have access to bb globaly. Not all tmp operations need to use this path only
/// operations that require tools to exist within a shell at that location.
pub fn get_tempdir() -> Result<TempDir> {
    let tmp = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target")
        .join("tmp");
    fs::create_dir_all(tmp.clone())?;
    Ok(TempDir::new_in(tmp)?)
}
