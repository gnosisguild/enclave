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
        "mkdir -p {} && cp -r {}/. {}",
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

// Async function to detect if we're using BSD sed
// You can test for BSD sed by checking if the --version flag is supported.
// GNU sed supports --version while BSD sed doesn't and will exit with an error.
async fn is_bsd_sed() -> bool {
    Command::new("sed")
        .arg("--version")
        .output()
        .await
        .map(|output| !output.status.success())
        .unwrap_or(false)
}

async fn apply_filter_to_files(base_path: impl AsRef<OsStr>, filter: &Filter) -> Result<()> {
    let is_bsd = is_bsd_sed().await;

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
        let sed_cmd = format!(
            "s|{}|{}|g",
            escape_sed_pattern(&filter.search_pattern),
            escape_sed_replacement(&filter.replacement)
        );

        println!("Running sed...");
        println!("> {}", sed_cmd);

        let mut cmd = Command::new("sed");

        // Check if we're on macOS (BSD sed) and add empty backup extension
        if is_bsd {
            // This is a quirk of BSD sed - we need to do the equivalent of:
            // sed -i '' pattern filename
            cmd.arg("-i").arg("");
        } else {
            // Normal sed is:
            // sed -i pattern filename
            cmd.arg("-i");
        }

        let sed_output = cmd
            .arg(sed_cmd)
            .arg(file_path)
            .output()
            .await
            .context("Failed to execute sed command")?;

        println!("{:?}", sed_output);

        if !sed_output.status.success() {
            let stderr = String::from_utf8_lossy(&sed_output.stderr);
            bail!("sed command failed on {}: {}", file_path, stderr);
        }
    }

    Ok(())
}

fn escape_sed_pattern(pattern: &str) -> String {
    pattern.replace("|", "\\|") // Only escape the pipe delimiter
}

fn escape_sed_replacement(replacement: &str) -> String {
    // Don't escape backslashes that are followed by digits (backreferences like \1, \2, etc.)
    let mut result = replacement.to_string();

    // First escape literal backslashes (but not backreferences)
    // This is tricky - we need to escape \ that aren't part of \1, \2, etc.

    // Simple approach: only escape the delimiter and ampersand
    result = result.replace("|", "\\|"); // Escape pipe delimiter
    result = result.replace("&", "\\&"); // Escape ampersand

    result
}
