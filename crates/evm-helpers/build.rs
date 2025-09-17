use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=FORCE_BUILD");

    assert!(Command::new("bash")
        .arg("./scripts/build_fixtures.sh")
        .status()
        .unwrap()
        .success());

    println!("cargo:rerun-if-changed=./scripts/build_fixtures.sh");
}
