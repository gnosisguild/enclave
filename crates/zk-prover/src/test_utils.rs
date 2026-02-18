// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::Path;

use tempfile::TempDir;

pub fn get_tempdir() -> anyhow::Result<TempDir> {
    Ok(TempDir::new_in(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp"),
    )?)
}
