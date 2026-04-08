# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-07 14:49:11 UTC

**Git Branch:** `main`  
**Git Commit:** `bae26bfb4e0d6673bff4783c4942384dfbef5a08`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile  | Execute | Prove   | Verify |
| ---------------------- | -------- | ------- | ------- | ------ |
| e_sm_share_computation | 430.82 s | 7.67 s  | 22.45 s | 0.03 s |
| pk                     | 12.68 s  | 0.54 s  | 1.05 s  | 0.03 s |
| share_decryption       | 164.72 s | 3.66 s  | 10.81 s | 0.02 s |
| share_encryption       | 538.42 s | 5.06 s  | 11.41 s | 0.09 s |
| sk_share_computation   | 176.93 s | 5.53 s  | 20.02 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 2468542 | 7.39M   | 31.59 MB     | 10.11 MB  | 3.59 KB | 15.88 KB   |
| pk                     | 10925   | 215.80K | 445.36 KB    | 952.11 KB | 3.59 KB | 15.88 KB   |
| share_decryption       | 928446  | 3.13M   | 12.72 MB     | 5.89 MB   | 3.59 KB | 15.88 KB   |
| share_encryption       | 1151876 | 3.19M   | 14.37 MB     | 14.19 MB  | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 1802613 | 4.90M   | 23.37 MB     | 6.12 MB   | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                      | Compile  | Execute | Prove   | Verify |
| ---------------------------- | -------- | ------- | ------- | ------ |
| decrypted_shares_aggregation | 1.50 s   | 0.70 s  | 0.81 s  | 0.02 s |
| pk_aggregation               | 126.53 s | 6.07 s  | 20.04 s | 0.02 s |
| pk_generation                | 379.45 s | 4.07 s  | 10.93 s | 0.02 s |
| share_decryption             | 478.26 s | 4.89 s  | 11.28 s | 0.16 s |
| user_data_encryption_ct0     | 118.00 s | 4.30 s  | 6.61 s  | 0.02 s |
| user_data_encryption_ct1     | 90.37 s  | 3.54 s  | 6.04 s  | 0.03 s |

#### Size & Circuit Metrics

| Circuit                      | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation | 59327   | 149.69K | 1.36 MB      | 201.40 KB | 3.59 KB | 15.88 KB   |
| pk_aggregation               | 1594721 | 5.28M   | 21.95 MB     | 10.22 MB  | 3.59 KB | 15.88 KB   |
| pk_generation                | 854628  | 3.09M   | 11.22 MB     | 12.89 MB  | 3.59 KB | 15.88 KB   |
| share_decryption             | 750193  | 3.02M   | 10.43 MB     | 18.04 MB  | 3.59 KB | 15.88 KB   |
| user_data_encryption_ct0     | 783630  | 1.91M   | 10.13 MB     | 12.55 MB  | 3.59 KB | 15.88 KB   |
| user_data_encryption_ct1     | 647104  | 1.61M   | 8.36 MB      | 10.42 MB  | 3.59 KB | 15.88 KB   |

### Config

#### Timing Metrics

| Circuit                 | Compile | Execute | Prove  | Verify |
| ----------------------- | ------- | ------- | ------ | ------ |
| validate_secure_configs | 0.31 s  | 0.31 s  | 0.00 s | 0.00 s |

#### Size & Circuit Metrics

| Circuit                 | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
| ----------------------- | ------- | ----- | ------------ | ------- | ------- | ---------- |
| validate_secure_configs | 0       | 0     | 21.09 KB     | 0 B     | 0 B     | 0 B        |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 430.82 s  |
| **Execution**        | 7.67 s    |
| **VK Generation**    | 10.77 s   |
| **Proof Generation** | 22.45 s   |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "2468542" |
| **Total Gates**      | "7387206" |
| **Circuit Size**     | 31.59 MB  |
| **Witness Size**     | 10.11 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 12.68 s   |
| **Execution**        | 0.54 s    |
| **VK Generation**    | 0.39 s    |
| **Proof Generation** | 1.05 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "10925"   |
| **Total Gates**      | "215804"  |
| **Circuit Size**     | 445.36 KB |
| **Witness Size**     | 952.11 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 164.72 s  |
| **Execution**        | 3.66 s    |
| **VK Generation**    | 4.53 s    |
| **Proof Generation** | 10.81 s   |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "928446"  |
| **Total Gates**      | "3127021" |
| **Circuit Size**     | 12.72 MB  |
| **Witness Size**     | 5.89 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 538.42 s  |
| **Execution**        | 5.06 s    |
| **VK Generation**    | 4.67 s    |
| **Proof Generation** | 11.41 s   |
| **Verification**     | 0.09 s    |
| **ACIR Opcodes**     | "1151876" |
| **Total Gates**      | "3194472" |
| **Circuit Size**     | 14.37 MB  |
| **Witness Size**     | 14.19 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 176.93 s  |
| **Execution**        | 5.53 s    |
| **VK Generation**    | 7.86 s    |
| **Proof Generation** | 20.02 s   |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "1802613" |
| **Total Gates**      | "4903487" |
| **Circuit Size**     | 23.37 MB  |
| **Witness Size**     | 6.12 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.50 s    |
| **Execution**        | 0.70 s    |
| **VK Generation**    | 0.32 s    |
| **Proof Generation** | 0.81 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "59327"   |
| **Total Gates**      | "149687"  |
| **Circuit Size**     | 1.36 MB   |
| **Witness Size**     | 201.40 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 126.53 s  |
| **Execution**        | 6.07 s    |
| **VK Generation**    | 8.17 s    |
| **Proof Generation** | 20.04 s   |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "1594721" |
| **Total Gates**      | "5284183" |
| **Circuit Size**     | 21.95 MB  |
| **Witness Size**     | 10.22 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 379.45 s  |
| **Execution**        | 4.07 s    |
| **VK Generation**    | 4.17 s    |
| **Proof Generation** | 10.93 s   |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "854628"  |
| **Total Gates**      | "3088306" |
| **Circuit Size**     | 11.22 MB  |
| **Witness Size**     | 12.89 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 478.26 s  |
| **Execution**        | 4.89 s    |
| **VK Generation**    | 4.24 s    |
| **Proof Generation** | 11.28 s   |
| **Verification**     | 0.16 s    |
| **ACIR Opcodes**     | "750193"  |
| **Total Gates**      | "3022296" |
| **Circuit Size**     | 10.43 MB  |
| **Witness Size**     | 18.04 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption_ct0

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 118.00 s  |
| **Execution**        | 4.30 s    |
| **VK Generation**    | 2.84 s    |
| **Proof Generation** | 6.61 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "783630"  |
| **Total Gates**      | "1911547" |
| **Circuit Size**     | 10.13 MB  |
| **Witness Size**     | 12.55 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption_ct1

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 90.37 s   |
| **Execution**        | 3.54 s    |
| **VK Generation**    | 2.43 s    |
| **Proof Generation** | 6.04 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "647104"  |
| **Total Gates**      | "1607045" |
| **Circuit Size**     | 8.36 MB   |
| **Witness Size**     | 10.42 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Config

#### validate_secure_configs

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.31 s   |
| **Execution**        | 0.31 s   |
| **VK Generation**    | 0.00 s   |
| **Proof Generation** | 0.00 s   |
| **Verification**     | 0.00 s   |
| **ACIR Opcodes**     | "0"      |
| **Total Gates**      | "0"      |
| **Circuit Size**     | 21.09 KB |
| **Witness Size**     | 0 B      |
| **VK Size**          | 0 B      |
| **Proof Size**       | 0 B      |

## System Information

### Hardware

- **CPU:** Apple M4 Pro
- **CPU Cores:** 14
- **RAM:** 48.00 GB
- **OS:** Darwin
- **Architecture:** arm64

### Software

- **Nargo Version:** nargo version = 1.0.0-beta.16 noirc version =
  1.0.0-beta.16+2d46fca7203545cbbfb31a0d0328de6c10a8db95 (git version hash:
  2d46fca7203545cbbfb31a0d0328de6c10a8db95, is dirty: false)
- **Barretenberg Version:** 3.0.0-nightly.20260102
