// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::BBPath;
use e3_zk_prover::{ZkBackend, ZkConfig};
use std::{env, path::PathBuf};
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
    let vk_evm_path = target_dir.join(format!("{circuit_name}.vk"));
    let vk_recursive_path = target_dir.join(format!("{circuit_name}.vk_recursive"));
    let vk_noir_path = target_dir.join(format!("{circuit_name}.vk_noir"));

    assert!(
        json_path.exists(),
        "compiled circuit not found: {} (run `pnpm build:circuits` to compile)",
        json_path.display()
    );
    assert!(
        vk_evm_path.exists(),
        "evm verification key not found: {} (run `pnpm build:circuits` to compile)",
        vk_evm_path.display()
    );

    // Set up the evm flavor directory (keccak VK)
    let evm_dir = backend
        .circuits_dir
        .join("evm")
        .join(group)
        .join(circuit_name);
    fs::create_dir_all(&evm_dir).await.unwrap();
    fs::copy(&json_path, evm_dir.join(format!("{circuit_name}.json")))
        .await
        .unwrap();
    fs::copy(&vk_evm_path, evm_dir.join(format!("{circuit_name}.vk")))
        .await
        .unwrap();

    // Set up the default flavor directory (noir-recursive-no-zk VK for wrapper/fold proofs)
    let default_dir = backend
        .circuits_dir
        .join("default")
        .join(group)
        .join(circuit_name);
    fs::create_dir_all(&default_dir).await.unwrap();
    fs::copy(&json_path, default_dir.join(format!("{circuit_name}.json")))
        .await
        .unwrap();
    // Use .vk_recursive (noir-recursive-no-zk) if available, otherwise fall back to .vk
    let default_vk_src = if vk_recursive_path.exists() {
        &vk_recursive_path
    } else {
        &vk_evm_path
    };
    fs::copy(
        default_vk_src,
        default_dir.join(format!("{circuit_name}.vk")),
    )
    .await
    .unwrap();

    // Set up the recursive flavor directory (noir-recursive VK for inner/base proofs)
    let recursive_dir = backend
        .circuits_dir
        .join("recursive")
        .join(group)
        .join(circuit_name);
    fs::create_dir_all(&recursive_dir).await.unwrap();
    fs::copy(
        &json_path,
        recursive_dir.join(format!("{circuit_name}.json")),
    )
    .await
    .unwrap();
    // Use .vk_noir (noir-recursive) if available, otherwise fall back to .vk_recursive, then .vk
    let recursive_vk_src = if vk_noir_path.exists() {
        &vk_noir_path
    } else if vk_recursive_path.exists() {
        &vk_recursive_path
    } else {
        &vk_evm_path
    };
    fs::copy(
        recursive_vk_src,
        recursive_dir.join(format!("{circuit_name}.vk")),
    )
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
    let backend = ZkBackend::new(bb_binary.clone(), circuits_dir.clone(), work_dir.clone());

    fs::create_dir_all(&backend.circuits_dir).await.unwrap();
    fs::create_dir_all(&backend.work_dir).await.unwrap();
    fs::create_dir_all(backend.base_dir.join("bin"))
        .await
        .unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(bb, &backend.bb_binary).unwrap();

    (backend, temp)
}

/// Lightweight backend for tests that need to override config (e.g. inject bad checksums).
pub fn test_backend(temp_path: &std::path::Path, config: ZkConfig) -> ZkBackend {
    let noir_dir = temp_path.join("noir");
    let bb_binary = match env::var("E3_CUSTOM_BB") {
        Ok(path) => BBPath::Custom(PathBuf::from(path)),
        Err(_) => BBPath::Default(noir_dir.join("bin").join("bb")),
    };
    let circuits_dir = noir_dir.join("circuits");
    let work_dir = noir_dir.join("work").join("test_node");
    ZkBackend::with_config(bb_binary, circuits_dir, work_dir, config)
}
