# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-15 11:38:26 UTC

**Git Branch:** `up/params`  
**Git Commit:** `6359bfdf5bc7a5c8ea59a494a78b00341c318cb1`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile  | Execute | Prove   | Verify |
| ---------------------- | -------- | ------- | ------- | ------ |
| e_sm_share_computation | 288.92 s | 5.88 s  | 20.08 s | 0.03 s |
| pk                     | 22.05 s  | 0.62 s  | 1.60 s  | 0.03 s |
| share_decryption       | 126.46 s | 2.96 s  | 9.76 s  | 0.02 s |
| share_encryption       | 778.88 s | 5.14 s  | 11.65 s | 0.09 s |
| sk_share_computation   | 135.62 s | 4.70 s  | 13.34 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness  | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | -------- | ------- | ---------- |
| e_sm_share_computation | 1859606 | 5.74M   | 23.81 MB     | 8.12 MB  | 3.59 KB | 15.88 KB   |
| pk                     | 14568   | 287.76K | 501.04 KB    | 1.12 MB  | 3.59 KB | 15.88 KB   |
| share_decryption       | 707266  | 2.56M   | 9.72 MB      | 4.98 MB  | 3.59 KB | 15.88 KB   |
| share_encryption       | 1167356 | 3.52M   | 14.61 MB     | 15.09 MB | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 1360248 | 3.88M   | 17.62 MB     | 5.05 MB  | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                      | Compile  | Execute | Prove   | Verify |
| ---------------------------- | -------- | ------- | ------- | ------ |
| decrypted_shares_aggregation | 1.35 s   | 0.66 s  | 0.54 s  | 0.02 s |
| pk_aggregation               | 106.23 s | 4.76 s  | 18.09 s | 0.02 s |
| pk_generation                | 264.52 s | 3.27 s  | 9.66 s  | 0.02 s |
| share_decryption             | 375.76 s | 3.90 s  | 10.14 s | 0.12 s |
| user_data_encryption_ct0     | 100.06 s | 3.36 s  | 6.39 s  | 0.03 s |
| user_data_encryption_ct1     | 73.23 s  | 2.78 s  | 5.46 s  | 0.02 s |

#### Size & Circuit Metrics

| Circuit                      | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation | 50592   | 128.31K | 1.25 MB      | 174.93 KB | 3.59 KB | 15.88 KB   |
| pk_aggregation               | 1217897 | 4.40M   | 16.86 MB     | 8.83 MB   | 3.59 KB | 15.88 KB   |
| pk_generation                | 669868  | 2.43M   | 8.87 MB      | 10.35 MB  | 3.59 KB | 15.88 KB   |
| share_decryption             | 579100  | 2.63M   | 8.10 MB      | 14.57 MB  | 3.59 KB | 15.88 KB   |
| user_data_encryption_ct0     | 626751  | 1.68M   | 8.12 MB      | 10.02 MB  | 3.59 KB | 15.88 KB   |
| user_data_encryption_ct1     | 506035  | 1.40M   | 6.59 MB      | 8.44 MB   | 3.59 KB | 15.88 KB   |

### Config

#### Timing Metrics

| Circuit                 | Compile | Execute | Prove  | Verify |
| ----------------------- | ------- | ------- | ------ | ------ |
| validate_secure_configs | 0.30 s  | 0.29 s  | 0.00 s | 0.00 s |

#### Size & Circuit Metrics

| Circuit                 | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
| ----------------------- | ------- | ----- | ------------ | ------- | ------- | ---------- |
| validate_secure_configs | 0       | 0     | 21.09 KB     | 0 B     | 0 B     | 0 B        |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 288.92 s  |
| **Execution**        | 5.88 s    |
| **VK Generation**    | 8.38 s    |
| **Proof Generation** | 20.08 s   |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "1859606" |
| **Total Gates**      | "5739750" |
| **Circuit Size**     | 23.81 MB  |
| **Witness Size**     | 8.12 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 22.05 s   |
| **Execution**        | 0.62 s    |
| **VK Generation**    | 0.48 s    |
| **Proof Generation** | 1.60 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "14568"   |
| **Total Gates**      | "287764"  |
| **Circuit Size**     | 501.04 KB |
| **Witness Size**     | 1.12 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 126.46 s  |
| **Execution**        | 2.96 s    |
| **VK Generation**    | 3.93 s    |
| **Proof Generation** | 9.76 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "707266"  |
| **Total Gates**      | "2564001" |
| **Circuit Size**     | 9.72 MB   |
| **Witness Size**     | 4.98 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 778.88 s  |
| **Execution**        | 5.14 s    |
| **VK Generation**    | 4.89 s    |
| **Proof Generation** | 11.65 s   |
| **Verification**     | 0.09 s    |
| **ACIR Opcodes**     | "1167356" |
| **Total Gates**      | "3520046" |
| **Circuit Size**     | 14.61 MB  |
| **Witness Size**     | 15.09 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 135.62 s  |
| **Execution**        | 4.70 s    |
| **VK Generation**    | 6.34 s    |
| **Proof Generation** | 13.34 s   |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "1360248" |
| **Total Gates**      | "3879330" |
| **Circuit Size**     | 17.62 MB  |
| **Witness Size**     | 5.05 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 1.35 s    |
| **Execution**        | 0.66 s    |
| **VK Generation**    | 0.25 s    |
| **Proof Generation** | 0.54 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "50592"   |
| **Total Gates**      | "128310"  |
| **Circuit Size**     | 1.25 MB   |
| **Witness Size**     | 174.93 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 106.23 s  |
| **Execution**        | 4.76 s    |
| **VK Generation**    | 6.46 s    |
| **Proof Generation** | 18.09 s   |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "1217897" |
| **Total Gates**      | "4395328" |
| **Circuit Size**     | 16.86 MB  |
| **Witness Size**     | 8.83 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 264.52 s  |
| **Execution**        | 3.27 s    |
| **VK Generation**    | 3.50 s    |
| **Proof Generation** | 9.66 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "669868"  |
| **Total Gates**      | "2432074" |
| **Circuit Size**     | 8.87 MB   |
| **Witness Size**     | 10.35 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 375.76 s  |
| **Execution**        | 3.90 s    |
| **VK Generation**    | 3.60 s    |
| **Proof Generation** | 10.14 s   |
| **Verification**     | 0.12 s    |
| **ACIR Opcodes**     | "579100"  |
| **Total Gates**      | "2628314" |
| **Circuit Size**     | 8.10 MB   |
| **Witness Size**     | 14.57 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption_ct0

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 100.06 s  |
| **Execution**        | 3.36 s    |
| **VK Generation**    | 2.53 s    |
| **Proof Generation** | 6.39 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "626751"  |
| **Total Gates**      | "1678200" |
| **Circuit Size**     | 8.12 MB   |
| **Witness Size**     | 10.02 MB  |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption_ct1

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 73.23 s   |
| **Execution**        | 2.78 s    |
| **VK Generation**    | 2.02 s    |
| **Proof Generation** | 5.46 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "506035"  |
| **Total Gates**      | "1398659" |
| **Circuit Size**     | 6.59 MB   |
| **Witness Size**     | 8.44 MB   |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Config

#### validate_secure_configs

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.30 s   |
| **Execution**        | 0.29 s   |
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
