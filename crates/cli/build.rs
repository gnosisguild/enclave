use std::process::Command;

fn main() {
    // Try to get local git SHA first
    let output = Command::new("git")
        .args(&["rev-parse", "--short=9", "HEAD"])
        .output();

    let git_sha = match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string(),
        _ => {
            // Fallback to remote commit hash
            get_remote_commit_hash().unwrap_or_else(|| "unknown".to_string())
        }
    };

    // Set environment variable for compilation
    println!("cargo:rustc-env=GIT_SHA={}", git_sha);
    // Rebuild if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}

fn get_remote_commit_hash() -> Option<String> {
    let output = Command::new("git")
        .args(&[
            "ls-remote",
            "https://github.com/gnosisguild/enclave",
            "refs/heads/main",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let commit_hash = stdout
        .split_whitespace()
        .next()?
        .chars()
        .take(9)
        .collect::<String>();

    if commit_hash.is_empty() {
        None
    } else {
        Some(commit_hash)
    }
}
