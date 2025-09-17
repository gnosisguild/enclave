use std::process::Command;

fn main() {
    // Run your bash script
    let output = Command::new("./scripts/build_fixtures.sh")
        .output()
        .expect("Failed to execute script");

    if !output.status.success() {
        panic!("Script failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Tell cargo to re-run this script if the bash script changes
    println!("cargo:rerun-if-changed=scripts/build_fixtures.sh");
}
