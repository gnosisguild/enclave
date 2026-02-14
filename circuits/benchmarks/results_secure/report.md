# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-13 14:38:47 UTC

**Git Branch:** `ref/circuits-comp`  
**Git Commit:** `2403461592f5c628cb22b493586995f79bb698e1`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile  | Execute | Prove   | Verify |
| ---------------------- | -------- | ------- | ------- | ------ |
| e_sm_share_computation | 1.94 s   | 9.97 s  | 38.67 s | 0.03 s |
| pk                     | 0.26 s   | 0.47 s  | 1.00 s  | 0.02 s |
| share_decryption       | 45.18 s  | 1.90 s  | 5.71 s  | 0.02 s |
| share_encryption       | 462.32 s | 5.14 s  | 11.85 s | 0.09 s |
| sk_share_computation   | 1.80 s   | 9.05 s  | 37.63 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 2949141 | 11.54M  | 39.14 MB     | 17.63 MB  | 3.59 KB | 15.88 KB   |
| pk                     | 10925   | 215.80K | 442.45 KB    | 952.12 KB | 3.59 KB | 15.88 KB   |
| share_decryption       | 81950   | 1.33M   | 2.65 MB      | 5.57 MB   | 3.59 KB | 15.88 KB   |
| share_encryption       | 1151876 | 3.20M   | 14.36 MB     | 14.19 MB  | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 2905804 | 10.72M  | 38.25 MB     | 15.36 MB  | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                         | Compile  | Execute | Prove   | Verify |
| ------------------------------- | -------- | ------- | ------- | ------ |
| decrypted_shares_aggregation_bn | 1.40 s   | 0.73 s  | 0.89 s  | 0.03 s |
| pk_aggregation                  | 153.74 s | 7.90 s  | 21.98 s | 0.03 s |
| pk_generation                   | 419.13 s | 4.95 s  | 22.79 s | 0.10 s |
| share_decryption                | 520.26 s | 6.24 s  | 13.80 s | 0.17 s |
| user_data_encryption            | 434.14 s | 7.88 s  | 15.04 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                         | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_bn | 61568   | 154.96K | 1.29 MB      | 194.16 KB | 3.59 KB | 15.88 KB   |
| pk_aggregation                  | 1572875 | 6.13M   | 23.79 MB     | 14.66 MB  | 3.59 KB | 15.88 KB   |
| pk_generation                   | 948955  | 3.49M   | 12.31 MB     | 16.86 MB  | 3.59 KB | 15.88 KB   |
| share_decryption                | 1012104 | 3.54M   | 12.98 MB     | 19.20 MB  | 3.59 KB | 15.88 KB   |
| user_data_encryption            | 1684299 | 4.02M   | 20.75 MB     | 23.82 MB  | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value      |
| -------------------- | ---------- |
| **Compilation**      | 1.94 s     |
| **Execution**        | 9.97 s     |
| **VK Generation**    | 16.31 s    |
| **Proof Generation** | 38.67 s    |
| **Verification**     | 0.03 s     |
| **ACIR Opcodes**     | "2949141"  |
| **Total Gates**      | "11539441" |
| **Circuit Size**     | 39.14 MB   |
| **Witness Size**     | 17.63 MB   |
| **VK Size**          | 3.59 KB    |
| **Proof Size**       | 15.88 KB   |

#### pk

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.26 s    |
| **Execution**        | 0.47 s    |
| **VK Generation**    | 0.34 s    |
| **Proof Generation** | 1.00 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "10925"   |
| **Total Gates**      | "215803"  |
| **Circuit Size**     | 442.45 KB |
| **Witness Size**     | 952.12 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 45.18 s   |
| **Execution**        | 1.90 s    |
| **VK Generation**    | 1.92 s    |
| **Proof Generation** | 5.71 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "81950"   |
| **Total Gates**      | "1327693" |
| **Circuit Size**     | 2.65 MB   |
| **Witness Size**     | 5.57 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 462.32 s  |
| **Execution**        | 5.14 s    |
| **VK Generation**    | 5.26 s    |
| **Proof Generation** | 11.85 s   |
| **Verification**     | 0.09 s    |
| **ACIR Opcodes**     | "1151876" |
| **Total Gates**      | "3204716" |
| **Circuit Size**     | 14.36 MB  |
| **Witness Size**     | 14.19 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value      |
| -------------------- | ---------- |
| **Compilation**      | 1.80 s     |
| **Execution**        | 9.05 s     |
| **VK Generation**    | 15.19 s    |
| **Proof Generation** | 37.63 s    |
| **Verification**     | 0.03 s     |
| **ACIR Opcodes**     | "2905804"  |
| **Total Gates**      | "10718698" |
| **Circuit Size**     | 38.25 MB   |
| **Witness Size**     | 15.36 MB   |
| **VK Size**          | 3.59 KB    |
| **Proof Size**       | 15.88 KB   |

### Threshold

#### decrypted_shares_aggregation_bn

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.40 s    |
| **Execution**        | 0.73 s    |
| **VK Generation**    | 0.35 s    |
| **Proof Generation** | 0.89 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "61568"   |
| **Total Gates**      | "154955"  |
| **Circuit Size**     | 1.29 MB   |
| **Witness Size**     | 194.16 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 153.74 s  |
| **Execution**        | 7.90 s    |
| **VK Generation**    | 9.48 s    |
| **Proof Generation** | 21.98 s   |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "1572875" |
| **Total Gates**      | "6130710" |
| **Circuit Size**     | 23.79 MB  |
| **Witness Size**     | 14.66 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 419.13 s  |
| **Execution**        | 4.95 s    |
| **VK Generation**    | 5.33 s    |
| **Proof Generation** | 22.79 s   |
| **Verification**     | 0.10 s    |
| **ACIR Opcodes**     | "948955"  |
| **Total Gates**      | "3485220" |
| **Circuit Size**     | 12.31 MB  |
| **Witness Size**     | 16.86 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 520.26 s  |
| **Execution**        | 6.24 s    |
| **VK Generation**    | 6.06 s    |
| **Proof Generation** | 13.80 s   |
| **Verification**     | 0.17 s    |
| **ACIR Opcodes**     | "1012104" |
| **Total Gates**      | "3543998" |
| **Circuit Size**     | 12.98 MB  |
| **Witness Size**     | 19.20 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 434.14 s  |
| **Execution**        | 7.88 s    |
| **VK Generation**    | 6.55 s    |
| **Proof Generation** | 15.04 s   |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "1684299" |
| **Total Gates**      | "4021683" |
| **Circuit Size**     | 20.75 MB  |
| **Witness Size**     | 23.82 MB  |
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
