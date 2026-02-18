use std::path::Path;

use tempfile::TempDir;

pub fn get_tempdir() -> anyhow::Result<TempDir> {
    Ok(TempDir::new_in(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp"),
    )?)
}
