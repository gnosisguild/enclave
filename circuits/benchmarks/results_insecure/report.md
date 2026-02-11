# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-11 09:57:32 UTC

**Git Branch:** `circuits/configs-benches`  
**Git Commit:** `eb62e90b8e36fedfd255a2dc8e6867296c2dc379`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 4.67 s  | 0.50 s  | 1.67 s | 0.03 s |
| e_sm_share_decryption  | 0.82 s  | 0.29 s  | 0.24 s | 0.02 s |
| e_sm_share_encryption  | 0.30 s  | 0.45 s  | 0.63 s | 0.03 s |
| pk                     | 0.34 s  | 0.26 s  | 0.12 s | 0.02 s |
| sk_share_computation   | 4.63 s  | 0.53 s  | 1.56 s | 0.02 s |
| sk_share_decryption    | 0.79 s  | 0.27 s  | 0.24 s | 0.02 s |
| sk_share_encryption    | 0.29 s  | 0.44 s  | 0.62 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 90956   | 328.74K | 1.39 MB      | 477.92 KB | 3.59 KB | 15.88 KB   |
| e_sm_share_decryption  | 3093    | 28.72K  | 158.28 KB    | 148.85 KB | 3.59 KB | 15.88 KB   |
| e_sm_share_encryption  | 47758   | 127.69K | 797.90 KB    | 512.26 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 87.84 KB     | 29.08 KB  | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 90827   | 326.14K | 1.38 MB      | 463.65 KB | 3.59 KB | 15.88 KB   |
| sk_share_decryption    | 3093    | 28.72K  | 158.27 KB    | 148.83 KB | 3.59 KB | 15.88 KB   |
| sk_share_encryption    | 47758   | 127.69K | 797.90 KB    | 512.48 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                          | Compile | Execute | Prove  | Verify |
| -------------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation_bn  | 0.30 s  | 0.50 s  | 0.52 s | 0.03 s |
| decrypted_shares_aggregation_mod | 0.27 s  | 0.32 s  | 0.46 s | 0.02 s |
| pk_aggregation                   | 2.32 s  | 0.44 s  | 0.90 s | 0.02 s |
| pk_generation                    | 1.95 s  | 0.38 s  | 0.51 s | 0.03 s |
| share_decryption                 | 1.85 s  | 0.39 s  | 0.53 s | 0.03 s |
| user_data_encryption             | 2.78 s  | 0.47 s  | 0.57 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                          | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| -------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_bn  | 40424   | 102.01K | 1.00 MB      | 104.64 KB | 3.59 KB | 15.88 KB   |
| decrypted_shares_aggregation_mod | 31544   | 80.74K  | 509.84 KB    | 77.56 KB  | 3.59 KB | 15.88 KB   |
| pk_aggregation                   | 47817   | 169.89K | 884.11 KB    | 360.79 KB | 3.59 KB | 15.88 KB   |
| pk_generation                    | 30019   | 65.61K  | 542.16 KB    | 446.26 KB | 3.59 KB | 15.88 KB   |
| share_decryption                 | 30570   | 85.48K  | 541.56 KB    | 522.92 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption             | 56601   | 106.72K | 847.68 KB    | 690.24 KB | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 4.67 s    |
| **Execution**        | 0.50 s    |
| **VK Generation**    | 0.57 s    |
| **Proof Generation** | 1.67 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "90956"   |
| **Total Gates**      | "328743"  |
| **Circuit Size**     | 1.39 MB   |
| **Witness Size**     | 477.92 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### e_sm_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.82 s    |
| **Execution**        | 0.29 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.24 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.28 KB |
| **Witness Size**     | 148.85 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### e_sm_share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.45 s    |
| **VK Generation**    | 0.27 s    |
| **Proof Generation** | 0.63 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127691"  |
| **Circuit Size**     | 797.90 KB |
| **Witness Size**     | 512.26 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.34 s   |
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

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 4.63 s    |
| **Execution**        | 0.53 s    |
| **VK Generation**    | 0.60 s    |
| **Proof Generation** | 1.56 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "90827"   |
| **Total Gates**      | "326138"  |
| **Circuit Size**     | 1.38 MB   |
| **Witness Size**     | 463.65 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.79 s    |
| **Execution**        | 0.27 s    |
| **VK Generation**    | 0.09 s    |
| **Proof Generation** | 0.24 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "3093"    |
| **Total Gates**      | "28720"   |
| **Circuit Size**     | 158.27 KB |
| **Witness Size**     | 148.83 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.29 s    |
| **Execution**        | 0.44 s    |
| **VK Generation**    | 0.26 s    |
| **Proof Generation** | 0.62 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127691"  |
| **Circuit Size**     | 797.90 KB |
| **Witness Size**     | 512.48 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation_bn

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.50 s    |
| **VK Generation**    | 0.23 s    |
| **Proof Generation** | 0.52 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "40424"   |
| **Total Gates**      | "102014"  |
| **Circuit Size**     | 1.00 MB   |
| **Witness Size**     | 104.64 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### decrypted_shares_aggregation_mod

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.27 s    |
| **Execution**        | 0.32 s    |
| **VK Generation**    | 0.18 s    |
| **Proof Generation** | 0.46 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "31544"   |
| **Total Gates**      | "80740"   |
| **Circuit Size**     | 509.84 KB |
| **Witness Size**     | 77.56 KB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 2.32 s    |
| **Execution**        | 0.44 s    |
| **VK Generation**    | 0.34 s    |
| **Proof Generation** | 0.90 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "47817"   |
| **Total Gates**      | "169890"  |
| **Circuit Size**     | 884.11 KB |
| **Witness Size**     | 360.79 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.95 s    |
| **Execution**        | 0.38 s    |
| **VK Generation**    | 0.16 s    |
| **Proof Generation** | 0.51 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30019"   |
| **Total Gates**      | "65606"   |
| **Circuit Size**     | 542.16 KB |
| **Witness Size**     | 446.26 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.85 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.53 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "30570"   |
| **Total Gates**      | "85478"   |
| **Circuit Size**     | 541.56 KB |
| **Witness Size**     | 522.92 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 2.78 s    |
| **Execution**        | 0.47 s    |
| **VK Generation**    | 0.22 s    |
| **Proof Generation** | 0.57 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "56601"   |
| **Total Gates**      | "106725"  |
| **Circuit Size**     | 847.68 KB |
| **Witness Size**     | 690.24 KB |
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
