# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-10 14:31:26 UTC

**Git Branch:** `circuits/configs-benches`  
**Git Commit:** `a59e54c02ae531cb9579c2fae86caab28c7e9efa`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 0.33 s  | 0.53 s  | 1.62 s | 0.03 s |
| e_sm_share_decryption  | 0.26 s  | 0.27 s  | 0.21 s | 0.02 s |
| e_sm_share_encryption  | 0.30 s  | 0.45 s  | 0.63 s | 0.03 s |
| pk                     | 0.25 s  | 0.27 s  | 0.12 s | 0.02 s |
| sk_share_computation   | 0.32 s  | 0.52 s  | 1.69 s | 0.02 s |
| sk_share_decryption    | 0.27 s  | 0.28 s  | 0.21 s | 0.02 s |
| sk_share_encryption    | 0.29 s  | 0.44 s  | 0.62 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 90956   | 328.74K | 1.39 MB      | 477.88 KB | 3.59 KB | 15.88 KB   |
| e_sm_share_decryption  | 1949    | 19.05K  | 129.17 KB    | 95.40 KB  | 3.59 KB | 15.88 KB   |
| e_sm_share_encryption  | 47758   | 127.69K | 797.90 KB    | 512.26 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 87.63 KB     | 29.09 KB  | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 90827   | 326.14K | 1.38 MB      | 463.66 KB | 3.59 KB | 15.88 KB   |
| sk_share_decryption    | 1949    | 19.05K  | 129.17 KB    | 95.45 KB  | 3.59 KB | 15.88 KB   |
| sk_share_encryption    | 47758   | 127.69K | 797.90 KB    | 512.48 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                          | Compile | Execute | Prove  | Verify |
| -------------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation_bn  | 0.30 s  | 0.50 s  | 0.52 s | 0.03 s |
| decrypted_shares_aggregation_mod | 0.28 s  | 0.33 s  | 0.47 s | 0.03 s |
| pk_aggregation                   | 0.28 s  | 0.41 s  | 0.86 s | 0.02 s |
| pk_generation                    | 0.27 s  | 0.39 s  | 0.50 s | 0.03 s |
| share_decryption                 | 0.28 s  | 0.39 s  | 0.56 s | 0.03 s |
| user_data_encryption             | 0.29 s  | 0.47 s  | 0.60 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                          | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| -------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_bn  | 40424   | 102.01K | 1.00 MB      | 104.64 KB | 3.59 KB | 15.88 KB   |
| decrypted_shares_aggregation_mod | 31544   | 80.74K  | 509.67 KB    | 77.58 KB  | 3.59 KB | 15.88 KB   |
| pk_aggregation                   | 46897   | 151.06K | 821.80 KB    | 278.60 KB | 3.59 KB | 15.88 KB   |
| pk_generation                    | 30019   | 65.61K  | 541.92 KB    | 445.47 KB | 3.59 KB | 15.88 KB   |
| share_decryption                 | 30570   | 85.48K  | 541.56 KB    | 522.85 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption             | 56601   | 106.72K | 847.43 KB    | 691.14 KB | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.33 s    |
| **Execution**        | 0.53 s    |
| **VK Generation**    | 0.60 s    |
| **Proof Generation** | 1.62 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | 90956     |
| **Total Gates**      | 328743    |
| **Circuit Size**     | 1.39 MB   |
| **Witness Size**     | 477.88 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### e_sm_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.26 s    |
| **Execution**        | 0.27 s    |
| **VK Generation**    | 0.07 s    |
| **Proof Generation** | 0.21 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 1949      |
| **Total Gates**      | 19049     |
| **Circuit Size**     | 129.17 KB |
| **Witness Size**     | 95.40 KB  |
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
| **ACIR Opcodes**     | 47758     |
| **Total Gates**      | 127691    |
| **Circuit Size**     | 797.90 KB |
| **Witness Size**     | 512.26 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.25 s   |
| **Execution**        | 0.27 s   |
| **VK Generation**    | 0.05 s   |
| **Proof Generation** | 0.12 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | 344      |
| **Total Gates**      | 6846     |
| **Circuit Size**     | 87.63 KB |
| **Witness Size**     | 29.09 KB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.52 s    |
| **VK Generation**    | 0.62 s    |
| **Proof Generation** | 1.69 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 90827     |
| **Total Gates**      | 326138    |
| **Circuit Size**     | 1.38 MB   |
| **Witness Size**     | 463.66 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.27 s    |
| **Execution**        | 0.28 s    |
| **VK Generation**    | 0.07 s    |
| **Proof Generation** | 0.21 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 1949      |
| **Total Gates**      | 19049     |
| **Circuit Size**     | 129.17 KB |
| **Witness Size**     | 95.45 KB  |
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
| **ACIR Opcodes**     | 47758     |
| **Total Gates**      | 127691    |
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
| **ACIR Opcodes**     | 40424     |
| **Total Gates**      | 102014    |
| **Circuit Size**     | 1.00 MB   |
| **Witness Size**     | 104.64 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### decrypted_shares_aggregation_mod

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.33 s    |
| **VK Generation**    | 0.19 s    |
| **Proof Generation** | 0.47 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | 31544     |
| **Total Gates**      | 80740     |
| **Circuit Size**     | 509.67 KB |
| **Witness Size**     | 77.58 KB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.41 s    |
| **VK Generation**    | 0.33 s    |
| **Proof Generation** | 0.86 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 46897     |
| **Total Gates**      | 151056    |
| **Circuit Size**     | 821.80 KB |
| **Witness Size**     | 278.60 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.27 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.17 s    |
| **Proof Generation** | 0.50 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | 30019     |
| **Total Gates**      | 65606     |
| **Circuit Size**     | 541.92 KB |
| **Witness Size**     | 445.47 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.28 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.20 s    |
| **Proof Generation** | 0.56 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | 30570     |
| **Total Gates**      | 85478     |
| **Circuit Size**     | 541.56 KB |
| **Witness Size**     | 522.85 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.29 s    |
| **Execution**        | 0.47 s    |
| **VK Generation**    | 0.23 s    |
| **Proof Generation** | 0.60 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 56601     |
| **Total Gates**      | 106725    |
| **Circuit Size**     | 847.43 KB |
| **Witness Size**     | 691.14 KB |
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
