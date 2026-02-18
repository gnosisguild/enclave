// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fs, path::Path};

use anyhow::Result;
use tempfile::TempDir;

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
