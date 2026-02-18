# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-17 15:08:16 UTC

**Git Branch:** `configs/fixqmt`  
**Git Commit:** `689e56cb90251b34e67af87cb7abfed03bedcd1c`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 3.29 s  | 0.54 s  | 1.63 s | 0.03 s |
| pk                     | 0.33 s  | 0.26 s  | 0.12 s | 0.02 s |
| share_decryption       | 0.71 s  | 0.29 s  | 0.23 s | 0.02 s |
| share_encryption       | 1.99 s  | 0.43 s  | 0.62 s | 0.03 s |
| sk_share_computation   | 3.18 s  | 0.51 s  | 1.62 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 90956   | 328.74K | 1.39 MB      | 477.88 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 87.84 KB     | 29.08 KB  | 3.59 KB | 15.88 KB   |
| share_decryption       | 3093    | 28.72K  | 158.27 KB    | 148.90 KB | 3.59 KB | 15.88 KB   |
| share_encryption       | 47758   | 127.69K | 798.14 KB    | 512.29 KB | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 90827   | 326.14K | 1.38 MB      | 463.75 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                          | Compile | Execute | Prove  | Verify |
| -------------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation_mod | 0.54 s  | 0.32 s  | 0.47 s | 0.03 s |
| pk_aggregation                   | 1.45 s  | 0.43 s  | 0.88 s | 0.02 s |
| pk_generation                    | 1.24 s  | 0.37 s  | 0.50 s | 0.03 s |
| share_decryption                 | 1.18 s  | 0.37 s  | 0.52 s | 0.03 s |
| user_data_encryption             | 2.01 s  | 0.47 s  | 0.61 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                          | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| -------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_mod | 31544   | 80.74K  | 509.84 KB    | 77.50 KB  | 3.59 KB | 15.88 KB   |
| pk_aggregation                   | 47817   | 169.89K | 884.11 KB    | 360.70 KB | 3.59 KB | 15.88 KB   |
| pk_generation                    | 30019   | 65.61K  | 542.16 KB    | 446.98 KB | 3.59 KB | 15.88 KB   |
| share_decryption                 | 22378   | 74.21K  | 460.26 KB    | 494.32 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption             | 56601   | 106.72K | 847.53 KB    | 691.33 KB | 3.59 KB | 15.88 KB   |

### Config

#### Timing Metrics

| Circuit | Compile | Execute | Prove | Verify |
| ------- | ------- | ------- | ----- | ------ |

#### Size & Circuit Metrics

| Circuit | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
| ------- | ------- | ----- | ------------ | ------- | ------- | ---------- |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 3.29 s    |
| **Execution**        | 0.54 s    |
| **VK Generation**    | 0.64 s    |
| **Proof Generation** | 1.63 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "90956"   |
| **Total Gates**      | "328743"  |
| **Circuit Size**     | 1.39 MB   |
| **Witness Size**     | 477.88 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.33 s   |
| **Execution**        | 0.26 s   |
| **VK Generation**    | 0.05 s   |
| **Proof Generation** | 0.12 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | "344"    |
| **Total Gates**      | "6846"   |
| **Circuit Size**     | 87.84 KB |
| **Witness Size**     | 29.08 KB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.71 s    |
| **Execution**        | 0.29 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.23 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.27 KB |
| **Witness Size**     | 148.90 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.99 s    |
| **Execution**        | 0.43 s    |
| **VK Generation**    | 0.26 s    |
| **Proof Generation** | 0.62 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127691"  |
| **Circuit Size**     | 798.14 KB |
| **Witness Size**     | 512.29 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 3.18 s    |
| **Execution**        | 0.51 s    |
| **VK Generation**    | 0.60 s    |
| **Proof Generation** | 1.62 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "90827"   |
| **Total Gates**      | "326138"  |
| **Circuit Size**     | 1.38 MB   |
| **Witness Size**     | 463.75 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation_mod

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.54 s    |
| **Execution**        | 0.32 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.47 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "31544"   |
| **Total Gates**      | "80740"   |
| **Circuit Size**     | 509.84 KB |
| **Witness Size**     | 77.50 KB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.45 s    |
| **Execution**        | 0.43 s    |
| **VK Generation**    | 0.35 s    |
| **Proof Generation** | 0.88 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "47817"   |
| **Total Gates**      | "169890"  |
| **Circuit Size**     | 884.11 KB |
| **Witness Size**     | 360.70 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.24 s    |
| **Execution**        | 0.37 s    |
| **VK Generation**    | 0.17 s    |
| **Proof Generation** | 0.50 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30019"   |
| **Total Gates**      | "65606"   |
| **Circuit Size**     | 542.16 KB |
| **Witness Size**     | 446.98 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.18 s    |
| **Execution**        | 0.37 s    |
| **VK Generation**    | 0.17 s    |
| **Proof Generation** | 0.52 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "22378"   |
| **Total Gates**      | "74214"   |
| **Circuit Size**     | 460.26 KB |
| **Witness Size**     | 494.32 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 2.01 s    |
| **Execution**        | 0.47 s    |
| **VK Generation**    | 0.24 s    |
| **Proof Generation** | 0.61 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "56601"   |
| **Total Gates**      | "106725"  |
| **Circuit Size**     | 847.53 KB |
| **Witness Size**     | 691.33 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Config

## System Information

### Hardware

- **CPU:** Apple M4 Pro
- **CPU Cores:** 14
- **RAM:** 48.00 GB
- **OS:** Darwin
- **Architecture:** arm64

### Software

- **Nargo Version:** nargo version = 1.0.0-beta.15 noirc version =
  1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663 (git version hash:
  83245db91dcf63420ef4bcbbd85b98f397fee663, is dirty: false)
- **Barretenberg Version:** 3.0.0-nightly.20251104
