// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::BBPath;
use e3_zk_prover::{ZkBackend, ZkConfig};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::{fs, process::Command};

/// Returns `None` when bb is not found — tests should skip gracefully.
pub async fn find_bb() -> Option<PathBuf> {
    if let Ok(output) = Command::new("which").arg("bb").output().await {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        for path in [
            format!("{}/.bb/bb", home),
            format!("{}/.nargo/bin/bb", home),
            format!("{}/.enclave/noir/bin/bb", home),
        ] {
            if std::path::Path::new(&path).exists() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

/// Root of the compiled circuit artifacts: `{workspace}/circuits/bin/`.
fn circuits_build_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("circuits")
        .join("bin")
}

pub async fn setup_compiled_circuit(backend: &ZkBackend, group: &str, circuit_name: &str) {
    let target_dir = circuits_build_root().join(group).join("target");
    let json_path = target_dir.join(format!("{circuit_name}.json"));
    let vk_path = target_dir.join(format!("{circuit_name}.vk"));

    assert!(
        json_path.exists(),
        "compiled circuit not found: {} (run `pnpm build:circuits` to compile)",
        json_path.display()
    );
    assert!(
        vk_path.exists(),
        "verification key not found: {} (run `pnpm build:circuits` to compile)",
        vk_path.display()
    );

    let circuit_dir = backend.circuits_dir.join(group).join(circuit_name);
    fs::create_dir_all(&circuit_dir).await.unwrap();
    fs::copy(&json_path, circuit_dir.join(format!("{circuit_name}.json")))
        .await
        .unwrap();
    fs::copy(&vk_path, circuit_dir.join(format!("{circuit_name}.vk")))
        .await
        .unwrap();
}

pub async fn find_anvil() -> bool {
    if let Ok(output) = Command::new("which").arg("anvil").output().await {
        if output.status.success() {
            return true;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let path = format!("{}/.foundry/bin/anvil", home);
        if std::path::Path::new(&path).exists() {
            return true;
        }
    }
    false
}

/// Creates a temp ZkBackend with the real bb binary symlinked in.
/// Caller must hold onto the returned TempDir or it gets cleaned up.
pub async fn setup_test_prover(bb: &PathBuf) -> (ZkBackend, TempDir) {
    let target_tmp = env!("CARGO_TARGET_TMPDIR");
    let temp = TempDir::new_in(target_tmp).unwrap();

    let temp_path = temp.path();
    let noir_dir = temp_path.join("noir");
    let bb_binary = BBPath::Default(noir_dir.join("bin").join("bb"));
    let circuits_dir = noir_dir.join("circuits");
    let work_dir = noir_dir.join("work").join("test_node");
    let backend = ZkBackend::new(
        bb_binary.clone(),
        circuits_dir.clone(),
        work_dir.clone(),
        ZkConfig::default(),
    );

    fs::create_dir_all(&backend.circuits_dir).await.unwrap();
    fs::create_dir_all(backend.circuits_dir.join("vk"))
        .await
        .unwrap();
    fs::create_dir_all(&backend.work_dir).await.unwrap();
    fs::create_dir_all(backend.base_dir.join("bin"))
        .await
        .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(bb, &backend.bb_binary).unwrap();

    (backend, temp)
}

/// Lightweight backend for tests that don't need a real bb binary.
pub fn test_backend(temp_path: &std::path::Path, config: ZkConfig) -> ZkBackend {
    let noir_dir = temp_path.join("noir");
    let bb_binary = BBPath::Default(noir_dir.join("bin").join("bb"));
    let circuits_dir = noir_dir.join("circuits");
    let work_dir = noir_dir.join("work").join("test_node");
    ZkBackend::new(bb_binary, circuits_dir, work_dir, config)
}
