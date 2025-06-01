use anyhow::{bail, Context, Result};
use std::{ffi::OsStr, path::Path};
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct Filter {
    pub glob_pattern: String,
    pub search_pattern: String,
    pub replacement: String,
}

impl Filter {
    pub fn new(glob_pattern: &str, search_pattern: &str, replacement: &str) -> Self {
        Filter {
            glob_pattern: glob_pattern.to_string(),
            search_pattern: search_pattern.to_string(),
            replacement: replacement.to_string(),
        }
    }
}

pub async fn copy_with_filters<P1, P2>(
    src_path: P1,
    dest_path: P2,
    filters: &[Filter],
) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let src_path = src_path.as_ref();
    let dest_path = dest_path.as_ref();

    // Create destination directory and copy contents in one shell command
    let combined_command = format!(
        "mkdir -p {} && cp -r {}/* {}",
        dest_path.to_string_lossy(),
        src_path.to_string_lossy(),
        dest_path.to_string_lossy()
    );

    let output = Command::new("sh")
        .arg("-c")
        .arg(&combined_command)
        .output()
        .await
        .context("Failed to execute mkdir and cp commands")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("mkdir and cp commands failed: {}", stderr);
    }

    // Then apply filters to matching files
    for filter in filters {
        apply_filter_to_files(dest_path.as_os_str(), filter).await?;
    }
    Ok(())
}

async fn apply_filter_to_files(base_path: impl AsRef<OsStr>, filter: &Filter) -> Result<()> {
    // Find files matching the glob pattern
    let find_output = Command::new("find")
        .arg(base_path)
        .arg("-name")
        .arg(&filter.glob_pattern)
        .arg("-type")
        .arg("f")
        .output()
        .await
        .context("Failed to execute find command")?;

    if !find_output.status.success() {
        let stderr = String::from_utf8_lossy(&find_output.stderr);
        bail!("find command failed: {}", stderr);
    }

    let files = String::from_utf8_lossy(&find_output.stdout);

    // Apply sed replacement to each matching file
    for file_path in files.lines().filter(|line| !line.is_empty()) {
        let sed_output = Command::new("sed")
            .arg("-i")
            .arg(format!(
                "s/{}/{}/g",
                escape_sed_pattern(&filter.search_pattern),
                escape_sed_replacement(&filter.replacement)
            ))
            .arg(file_path)
            .output()
            .await
            .context("Failed to execute sed command")?;

        if !sed_output.status.success() {
            let stderr = String::from_utf8_lossy(&sed_output.stderr);
            bail!("sed command failed on {}: {}", file_path, stderr);
        }
    }

    Ok(())
}

fn escape_sed_pattern(pattern: &str) -> String {
    // Escape special sed characters in the search pattern
    pattern
        .replace('/', r"\/")
        .replace('&', r"\&")
        .replace('\\', r"\\")
}

fn escape_sed_replacement(replacement: &str) -> String {
    // Escape special sed characters in the replacement string
    replacement
        .replace('/', r"\/")
        .replace('&', r"\&")
        .replace('\\', r"\\")
}
