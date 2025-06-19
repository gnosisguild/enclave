use std::process::Command;

fn main() {
    // Get git SHA
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();

    let git_sha = match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string(),
        _ => "unknown".to_string(),
    };

    // Set environment variable for compilation
    println!("cargo:rustc-env=GIT_SHA={}", git_sha);

    // Rebuild if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
