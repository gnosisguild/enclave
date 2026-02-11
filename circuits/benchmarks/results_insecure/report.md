# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-11 10:04:35 UTC

**Git Branch:** `circuits/configs-benches`  
**Git Commit:** `783e213aee09c9317df9d47a4ae1b3037b2dfad8`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 0.30 s  | 0.50 s  | 1.53 s | 0.02 s |
| e_sm_share_decryption  | 0.25 s  | 0.28 s  | 0.23 s | 0.02 s |
| pk                     | 0.25 s  | 0.25 s  | 0.12 s | 0.02 s |
| sk_share_computation   | 0.30 s  | 0.50 s  | 1.57 s | 0.02 s |
| sk_share_decryption    | 0.25 s  | 0.28 s  | 0.23 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 90956   | 328.74K | 1.39 MB      | 477.80 KB | 3.59 KB | 15.88 KB   |
| e_sm_share_decryption  | 3093    | 28.72K  | 158.28 KB    | 148.92 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 87.84 KB     | 29.09 KB  | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 90827   | 326.14K | 1.38 MB      | 463.64 KB | 3.59 KB | 15.88 KB   |
| sk_share_decryption    | 3093    | 28.72K  | 158.27 KB    | 148.89 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                          | Compile | Execute | Prove  | Verify |
| -------------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation_mod | 0.27 s  | 0.32 s  | 0.47 s | 0.03 s |
| pk_aggregation                   | 0.28 s  | 0.43 s  | 0.99 s | 0.03 s |
| pk_generation                    | 0.27 s  | 0.37 s  | 0.48 s | 0.03 s |
| share_decryption                 | 0.28 s  | 0.39 s  | 0.53 s | 0.03 s |
| user_data_encryption             | 0.29 s  | 0.46 s  | 0.58 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                          | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| -------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_mod | 31544   | 80.74K  | 509.84 KB    | 77.56 KB  | 3.59 KB | 15.88 KB   |
| pk_aggregation                   | 47817   | 169.89K | 884.11 KB    | 360.78 KB | 3.59 KB | 15.88 KB   |
| pk_generation                    | 30019   | 65.61K  | 542.16 KB    | 446.29 KB | 3.59 KB | 15.88 KB   |
| share_decryption                 | 30570   | 85.48K  | 541.56 KB    | 522.85 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption             | 56601   | 106.72K | 847.68 KB    | 691.42 KB | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.50 s    |
| **VK Generation**    | 0.58 s    |
| **Proof Generation** | 1.53 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "90956"   |
| **Total Gates**      | "328743"  |
| **Circuit Size**     | 1.39 MB   |
| **Witness Size**     | 477.80 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### e_sm_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.25 s    |
| **Execution**        | 0.28 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.23 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.28 KB |
| **Witness Size**     | 148.92 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.25 s   |
| **Execution**        | 0.25 s   |
| **VK Generation**    | 0.05 s   |
| **Proof Generation** | 0.12 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | "344"    |
| **Total Gates**      | "6846"   |
| **Circuit Size**     | 87.84 KB |
| **Witness Size**     | 29.09 KB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.50 s    |
| **VK Generation**    | 0.57 s    |
| **Proof Generation** | 1.57 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "90827"   |
| **Total Gates**      | "326138"  |
| **Circuit Size**     | 1.38 MB   |
| **Witness Size**     | 463.64 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.25 s    |
| **Execution**        | 0.28 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.23 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.27 KB |
| **Witness Size**     | 148.89 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation_mod

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.27 s    |
| **Execution**        | 0.32 s    |
| **VK Generation**    | 0.18 s    |
| **Proof Generation** | 0.47 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "31544"   |
| **Total Gates**      | "80740"   |
| **Circuit Size**     | 509.84 KB |
| **Witness Size**     | 77.56 KB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.43 s    |
| **VK Generation**    | 0.34 s    |
| **Proof Generation** | 0.99 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47817"   |
| **Total Gates**      | "169890"  |
| **Circuit Size**     | 884.11 KB |
| **Witness Size**     | 360.78 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.27 s    |
| **Execution**        | 0.37 s    |
| **VK Generation**    | 0.16 s    |
| **Proof Generation** | 0.48 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30019"   |
| **Total Gates**      | "65606"   |
| **Circuit Size**     | 542.16 KB |
| **Witness Size**     | 446.29 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.53 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30570"   |
| **Total Gates**      | "85478"   |
| **Circuit Size**     | 541.56 KB |
| **Witness Size**     | 522.85 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.29 s    |
| **Execution**        | 0.46 s    |
| **VK Generation**    | 0.22 s    |
| **Proof Generation** | 0.58 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "56601"   |
| **Total Gates**      | "106725"  |
| **Circuit Size**     | 847.68 KB |
| **Witness Size**     | 691.42 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

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
