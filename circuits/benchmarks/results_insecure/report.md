# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-12 16:42:19 UTC

**Git Branch:** `refactor/input-computation`  
**Git Commit:** `2b72de5d2b119a672a6da46c355db03862614ac7`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 0.23 s  | 0.42 s  | 1.71 s | 0.03 s |
| pk                     | 0.19 s  | 0.19 s  | 0.12 s | 0.02 s |
| share_decryption       | 0.61 s  | 0.23 s  | 0.30 s | 0.03 s |
| share_encryption       | 0.22 s  | 0.36 s  | 0.65 s | 0.03 s |
| sk_share_computation   | 0.23 s  | 0.42 s  | 1.71 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 90956   | 328.74K | 1.39 MB      | 477.83 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 87.81 KB     | 29.06 KB  | 3.59 KB | 15.88 KB   |
| share_decryption       | 3093    | 28.72K  | 158.24 KB    | 148.90 KB | 3.59 KB | 15.88 KB   |
| share_encryption       | 47758   | 127.69K | 798.18 KB    | 512.13 KB | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 90827   | 326.14K | 1.38 MB      | 463.68 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                          | Compile | Execute | Prove  | Verify |
| -------------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation_mod | 0.21 s  | 0.25 s  | 0.51 s | 0.03 s |
| pk_aggregation                   | 0.22 s  | 0.37 s  | 0.95 s | 0.03 s |
| pk_generation                    | 0.21 s  | 0.31 s  | 0.55 s | 0.03 s |
| share_decryption                 | 0.21 s  | 0.33 s  | 0.58 s | 0.03 s |
| user_data_encryption             | 0.23 s  | 0.40 s  | 0.64 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                          | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| -------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_mod | 31544   | 80.74K  | 509.82 KB    | 77.42 KB  | 3.59 KB | 15.88 KB   |
| pk_aggregation                   | 47817   | 169.89K | 884.07 KB    | 360.64 KB | 3.59 KB | 15.88 KB   |
| pk_generation                    | 30019   | 65.61K  | 542.13 KB    | 446.90 KB | 3.59 KB | 15.88 KB   |
| share_decryption                 | 30570   | 85.48K  | 541.52 KB    | 522.88 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption             | 56601   | 106.72K | 847.64 KB    | 690.54 KB | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.23 s    |
| **Execution**        | 0.42 s    |
| **VK Generation**    | 0.59 s    |
| **Proof Generation** | 1.71 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "90956"   |
| **Total Gates**      | "328743"  |
| **Circuit Size**     | 1.39 MB   |
| **Witness Size**     | 477.83 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.19 s   |
| **Execution**        | 0.19 s   |
| **VK Generation**    | 0.05 s   |
| **Proof Generation** | 0.12 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | "344"    |
| **Total Gates**      | "6846"   |
| **Circuit Size**     | 87.81 KB |
| **Witness Size**     | 29.06 KB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.61 s    |
| **Execution**        | 0.23 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.30 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.24 KB |
| **Witness Size**     | 148.90 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.22 s    |
| **Execution**        | 0.36 s    |
| **VK Generation**    | 0.26 s    |
| **Proof Generation** | 0.65 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127691"  |
| **Circuit Size**     | 798.18 KB |
| **Witness Size**     | 512.13 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.23 s    |
| **Execution**        | 0.42 s    |
| **VK Generation**    | 0.62 s    |
| **Proof Generation** | 1.71 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "90827"   |
| **Total Gates**      | "326138"  |
| **Circuit Size**     | 1.38 MB   |
| **Witness Size**     | 463.68 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation_mod

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.21 s    |
| **Execution**        | 0.25 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.51 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "31544"   |
| **Total Gates**      | "80740"   |
| **Circuit Size**     | 509.82 KB |
| **Witness Size**     | 77.42 KB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.22 s    |
| **Execution**        | 0.37 s    |
| **VK Generation**    | 0.36 s    |
| **Proof Generation** | 0.95 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47817"   |
| **Total Gates**      | "169890"  |
| **Circuit Size**     | 884.07 KB |
| **Witness Size**     | 360.64 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.21 s    |
| **Execution**        | 0.31 s    |
| **VK Generation**    | 0.18 s    |
| **Proof Generation** | 0.55 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30019"   |
| **Total Gates**      | "65606"   |
| **Circuit Size**     | 542.13 KB |
| **Witness Size**     | 446.90 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.21 s    |
| **Execution**        | 0.33 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.58 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30570"   |
| **Total Gates**      | "85478"   |
| **Circuit Size**     | 541.52 KB |
| **Witness Size**     | 522.88 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.23 s    |
| **Execution**        | 0.40 s    |
| **VK Generation**    | 0.24 s    |
| **Proof Generation** | 0.64 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "56601"   |
| **Total Gates**      | "106725"  |
| **Circuit Size**     | 847.64 KB |
| **Witness Size**     | 690.54 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

## System Information

### Hardware

- **CPU:** Apple M4 Pro
- **CPU Cores:** 12
- **RAM:** 24.00 GB
- **OS:** Darwin
- **Architecture:** arm64

### Software

- **Nargo Version:** nargo version = 1.0.0-beta.15 noirc version =
  1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663 (git version hash:
  83245db91dcf63420ef4bcbbd85b98f397fee663, is dirty: false)
- **Barretenberg Version:** 3.0.0-nightly.20251104
