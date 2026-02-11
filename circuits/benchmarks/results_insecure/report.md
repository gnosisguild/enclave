# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-11 17:03:46 UTC

**Git Branch:** `main`  
**Git Commit:** `286813a7ec2d8a0edeb34ae723bb06c38049b7af`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 0.32 s  | 0.53 s  | 1.64 s | 0.03 s |
| e_sm_share_decryption  | 0.28 s  | 0.30 s  | 0.24 s | 0.02 s |
| e_sm_share_encryption  | 2.73 s  | 0.45 s  | 0.62 s | 0.03 s |
| pk                     | 0.26 s  | 0.27 s  | 0.12 s | 0.02 s |
| sk_share_computation   | 0.33 s  | 0.53 s  | 1.64 s | 0.02 s |
| sk_share_decryption    | 0.28 s  | 0.31 s  | 0.24 s | 0.02 s |
| sk_share_encryption    | 2.68 s  | 0.45 s  | 0.63 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 90956   | 328.74K | 1.39 MB      | 477.90 KB | 3.59 KB | 15.88 KB   |
| e_sm_share_decryption  | 3093    | 28.72K  | 158.28 KB    | 148.87 KB | 3.59 KB | 15.88 KB   |
| e_sm_share_encryption  | 47758   | 127.69K | 798.23 KB    | 512.41 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 87.84 KB     | 29.09 KB  | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 90827   | 326.14K | 1.38 MB      | 463.59 KB | 3.59 KB | 15.88 KB   |
| sk_share_decryption    | 3093    | 28.72K  | 158.27 KB    | 148.89 KB | 3.59 KB | 15.88 KB   |
| sk_share_encryption    | 47758   | 127.69K | 798.23 KB    | 512.31 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                          | Compile | Execute | Prove  | Verify |
| -------------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation_mod | 0.29 s  | 0.35 s  | 0.48 s | 0.03 s |
| pk_aggregation                   | 0.30 s  | 0.46 s  | 0.98 s | 0.03 s |
| pk_generation                    | 0.29 s  | 0.39 s  | 0.50 s | 0.03 s |
| share_decryption                 | 0.30 s  | 0.43 s  | 0.56 s | 0.03 s |
| user_data_encryption             | 0.31 s  | 0.51 s  | 0.60 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                          | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| -------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_mod | 31544   | 80.74K  | 509.84 KB    | 77.52 KB  | 3.59 KB | 15.88 KB   |
| pk_aggregation                   | 47817   | 169.89K | 884.11 KB    | 360.83 KB | 3.59 KB | 15.88 KB   |
| pk_generation                    | 30019   | 65.61K  | 542.16 KB    | 447.07 KB | 3.59 KB | 15.88 KB   |
| share_decryption                 | 30570   | 85.48K  | 541.56 KB    | 522.91 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption             | 56601   | 106.72K | 847.68 KB    | 691.27 KB | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.53 s    |
| **VK Generation**    | 0.61 s    |
| **Proof Generation** | 1.64 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "90956"   |
| **Total Gates**      | "328743"  |
| **Circuit Size**     | 1.39 MB   |
| **Witness Size**     | 477.90 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### e_sm_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.30 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.24 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.28 KB |
| **Witness Size**     | 148.87 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### e_sm_share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 2.73 s    |
| **Execution**        | 0.45 s    |
| **VK Generation**    | 0.26 s    |
| **Proof Generation** | 0.62 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127691"  |
| **Circuit Size**     | 798.23 KB |
| **Witness Size**     | 512.41 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.26 s   |
| **Execution**        | 0.27 s   |
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
| **Compilation**      | 0.33 s    |
| **Execution**        | 0.53 s    |
| **VK Generation**    | 0.61 s    |
| **Proof Generation** | 1.64 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "90827"   |
| **Total Gates**      | "326138"  |
| **Circuit Size**     | 1.38 MB   |
| **Witness Size**     | 463.59 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.31 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.24 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.27 KB |
| **Witness Size**     | 148.89 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 2.68 s    |
| **Execution**        | 0.45 s    |
| **VK Generation**    | 0.26 s    |
| **Proof Generation** | 0.63 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127691"  |
| **Circuit Size**     | 798.23 KB |
| **Witness Size**     | 512.31 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation_mod

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.29 s    |
| **Execution**        | 0.35 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.48 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "31544"   |
| **Total Gates**      | "80740"   |
| **Circuit Size**     | 509.84 KB |
| **Witness Size**     | 77.52 KB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.46 s    |
| **VK Generation**    | 0.36 s    |
| **Proof Generation** | 0.98 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47817"   |
| **Total Gates**      | "169890"  |
| **Circuit Size**     | 884.11 KB |
| **Witness Size**     | 360.83 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.29 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.17 s    |
| **Proof Generation** | 0.50 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30019"   |
| **Total Gates**      | "65606"   |
| **Circuit Size**     | 542.16 KB |
| **Witness Size**     | 447.07 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.43 s    |
| **VK Generation**    | 0.20 s    |
| **Proof Generation** | 0.56 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30570"   |
| **Total Gates**      | "85478"   |
| **Circuit Size**     | 541.56 KB |
| **Witness Size**     | 522.91 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.31 s    |
| **Execution**        | 0.51 s    |
| **VK Generation**    | 0.24 s    |
| **Proof Generation** | 0.60 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "56601"   |
| **Total Gates**      | "106725"  |
| **Circuit Size**     | 847.68 KB |
| **Witness Size**     | 691.27 KB |
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
