# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-11 09:44:15 UTC

**Git Branch:** `circuits/configs-benches`  
**Git Commit:** `eb62e90b8e36fedfd255a2dc8e6867296c2dc379`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile  | Execute | Prove   | Verify |
| ---------------------- | -------- | ------- | ------- | ------ |
| e_sm_share_computation | 744.00 s | 10.10 s | 40.60 s | 0.03 s |
| e_sm_share_decryption  | 31.68 s  | 1.25 s  | 3.33 s  | 0.02 s |
| e_sm_share_encryption  | 473.70 s | 5.09 s  | 12.15 s | 0.09 s |
| pk                     | 10.58 s  | 0.48 s  | 1.11 s  | 0.02 s |
| sk_share_computation   | 536.29 s | 9.23 s  | 38.13 s | 0.03 s |
| sk_share_decryption    | 32.84 s  | 1.26 s  | 3.32 s  | 0.02 s |
| sk_share_encryption    | 498.52 s | 5.18 s  | 12.07 s | 0.09 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 2949141 | 11.54M  | 39.14 MB     | 17.63 MB  | 3.59 KB | 15.88 KB   |
| e_sm_share_decryption  | 51902   | 879.66K | 1.70 MB      | 3.55 MB   | 3.59 KB | 15.88 KB   |
| e_sm_share_encryption  | 1151876 | 3.20M   | 14.36 MB     | 14.19 MB  | 3.59 KB | 15.88 KB   |
| pk                     | 10925   | 215.80K | 442.45 KB    | 952.21 KB | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 2905804 | 10.72M  | 38.25 MB     | 15.36 MB  | 3.59 KB | 15.88 KB   |
| sk_share_decryption    | 51902   | 879.66K | 1.70 MB      | 3.55 MB   | 3.59 KB | 15.88 KB   |
| sk_share_encryption    | 1151876 | 3.20M   | 14.36 MB     | 14.19 MB  | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                         | Compile  | Execute | Prove   | Verify |
| ------------------------------- | -------- | ------- | ------- | ------ |
| decrypted_shares_aggregation_bn | 0.30 s   | 0.58 s  | 0.80 s  | 0.02 s |
| pk_aggregation                  | 116.13 s | 6.22 s  | 20.25 s | 0.02 s |
| pk_generation                   | 388.08 s | 4.88 s  | 12.30 s | 0.09 s |
| share_decryption                | 430.14 s | 5.55 s  | 12.41 s | 0.16 s |
| user_data_encryption            | 409.30 s | 7.78 s  | 13.37 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                         | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ------------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation_bn | 61568   | 154.96K | 1.29 MB      | 194.35 KB | 3.59 KB | 15.88 KB   |
| pk_aggregation                  | 1529181 | 5.27M   | 21.69 MB     | 11.06 MB  | 3.59 KB | 15.88 KB   |
| pk_generation                   | 948955  | 3.49M   | 12.31 MB     | 16.86 MB  | 3.59 KB | 15.88 KB   |
| share_decryption                | 1012104 | 3.54M   | 12.98 MB     | 19.20 MB  | 3.59 KB | 15.88 KB   |
| user_data_encryption            | 1684299 | 4.02M   | 20.75 MB     | 23.82 MB  | 3.59 KB | 15.88 KB   |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 744.00 s |
| **Execution**        | 10.10 s  |
| **VK Generation**    | 16.53 s  |
| **Proof Generation** | 40.60 s  |
| **Verification**     | 0.03 s   |
| **ACIR Opcodes**     | 2949141  |
| **Total Gates**      | 11539441 |
| **Circuit Size**     | 39.14 MB |
| **Witness Size**     | 17.63 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### e_sm_share_decryption

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 31.68 s  |
| **Execution**        | 1.25 s   |
| **VK Generation**    | 1.25 s   |
| **Proof Generation** | 3.33 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | 51902    |
| **Total Gates**      | 879661   |
| **Circuit Size**     | 1.70 MB  |
| **Witness Size**     | 3.55 MB  |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### e_sm_share_encryption

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 473.70 s |
| **Execution**        | 5.09 s   |
| **VK Generation**    | 5.28 s   |
| **Proof Generation** | 12.15 s  |
| **Verification**     | 0.09 s   |
| **ACIR Opcodes**     | 1151876  |
| **Total Gates**      | 3204716  |
| **Circuit Size**     | 14.36 MB |
| **Witness Size**     | 14.19 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### pk

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 10.58 s   |
| **Execution**        | 0.48 s    |
| **VK Generation**    | 0.38 s    |
| **Proof Generation** | 1.11 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 10925     |
| **Total Gates**      | 215803    |
| **Circuit Size**     | 442.45 KB |
| **Witness Size**     | 952.21 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 536.29 s |
| **Execution**        | 9.23 s   |
| **VK Generation**    | 15.57 s  |
| **Proof Generation** | 38.13 s  |
| **Verification**     | 0.03 s   |
| **ACIR Opcodes**     | 2905804  |
| **Total Gates**      | 10718698 |
| **Circuit Size**     | 38.25 MB |
| **Witness Size**     | 15.36 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### sk_share_decryption

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 32.84 s  |
| **Execution**        | 1.26 s   |
| **VK Generation**    | 1.28 s   |
| **Proof Generation** | 3.32 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | 51902    |
| **Total Gates**      | 879661   |
| **Circuit Size**     | 1.70 MB  |
| **Witness Size**     | 3.55 MB  |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### sk_share_encryption

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 498.52 s |
| **Execution**        | 5.18 s   |
| **VK Generation**    | 5.46 s   |
| **Proof Generation** | 12.07 s  |
| **Verification**     | 0.09 s   |
| **ACIR Opcodes**     | 1151876  |
| **Total Gates**      | 3204716  |
| **Circuit Size**     | 14.36 MB |
| **Witness Size**     | 14.19 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

### Threshold

#### decrypted_shares_aggregation_bn

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.30 s    |
| **Execution**        | 0.58 s    |
| **VK Generation**    | 0.32 s    |
| **Proof Generation** | 0.80 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | 61568     |
| **Total Gates**      | 154955    |
| **Circuit Size**     | 1.29 MB   |
| **Witness Size**     | 194.35 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 116.13 s |
| **Execution**        | 6.22 s   |
| **VK Generation**    | 8.28 s   |
| **Proof Generation** | 20.25 s  |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | 1529181  |
| **Total Gates**      | 5267720  |
| **Circuit Size**     | 21.69 MB |
| **Witness Size**     | 11.06 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### pk_generation

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 388.08 s |
| **Execution**        | 4.88 s   |
| **VK Generation**    | 5.17 s   |
| **Proof Generation** | 12.30 s  |
| **Verification**     | 0.09 s   |
| **ACIR Opcodes**     | 948955   |
| **Total Gates**      | 3485220  |
| **Circuit Size**     | 12.31 MB |
| **Witness Size**     | 16.86 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### share_decryption

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 430.14 s |
| **Execution**        | 5.55 s   |
| **VK Generation**    | 5.37 s   |
| **Proof Generation** | 12.41 s  |
| **Verification**     | 0.16 s   |
| **ACIR Opcodes**     | 1012104  |
| **Total Gates**      | 3543998  |
| **Circuit Size**     | 12.98 MB |
| **Witness Size**     | 19.20 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### user_data_encryption

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 409.30 s |
| **Execution**        | 7.78 s   |
| **VK Generation**    | 6.35 s   |
| **Proof Generation** | 13.37 s  |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | 1684299  |
| **Total Gates**      | 4021683  |
| **Circuit Size**     | 20.75 MB |
| **Witness Size**     | 23.82 MB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

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
