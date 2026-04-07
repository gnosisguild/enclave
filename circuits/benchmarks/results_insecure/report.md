# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-07 13:56:44 UTC

**Git Branch:** `main`  
**Git Commit:** `bae26bfb4e0d6673bff4783c4942384dfbef5a08`

---

## Summary

### DKG

#### Timing Metrics

| Circuit                | Compile | Execute | Prove  | Verify |
| ---------------------- | ------- | ------- | ------ | ------ |
| e_sm_share_computation | 0.34 s  | 0.49 s  | 0.89 s | 0.02 s |
| pk                     | 0.30 s  | 0.31 s  | 0.12 s | 0.02 s |
| share_decryption       | 0.32 s  | 0.39 s  | 0.51 s | 0.02 s |
| share_encryption       | 0.33 s  | 0.48 s  | 0.60 s | 0.03 s |
| sk_share_computation   | 0.33 s  | 0.45 s  | 0.81 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit                | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| e_sm_share_computation | 75649   | 198.35K | 1.16 MB      | 222.06 KB | 3.59 KB | 15.88 KB   |
| pk                     | 344     | 6.85K   | 90.54 KB     | 29.07 KB  | 3.59 KB | 15.88 KB   |
| share_decryption       | 28577   | 92.52K  | 538.83 KB    | 151.95 KB | 3.59 KB | 15.88 KB   |
| share_encryption       | 47758   | 127.05K | 802.98 KB    | 512.11 KB | 3.59 KB | 15.88 KB   |
| sk_share_computation   | 56018   | 142.62K | 936.72 KB    | 165.20 KB | 3.59 KB | 15.88 KB   |

### Threshold

#### Timing Metrics

| Circuit                      | Compile | Execute | Prove  | Verify |
| ---------------------------- | ------- | ------- | ------ | ------ |
| decrypted_shares_aggregation | 0.33 s  | 0.58 s  | 0.51 s | 0.02 s |
| pk_aggregation               | 0.34 s  | 0.45 s  | 0.83 s | 0.02 s |
| pk_generation                | 0.32 s  | 0.40 s  | 0.35 s | 0.02 s |
| share_decryption             | 0.32 s  | 0.43 s  | 0.52 s | 0.03 s |
| user_data_encryption_ct0     | 0.32 s  | 0.41 s  | 0.33 s | 0.03 s |
| user_data_encryption_ct1     | 0.32 s  | 0.39 s  | 0.32 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit                      | Opcodes | Gates   | Circuit Size | Witness   | VK Size | Proof Size |
| ---------------------------- | ------- | ------- | ------------ | --------- | ------- | ---------- |
| decrypted_shares_aggregation | 41673   | 104.27K | 1.12 MB      | 111.14 KB | 3.59 KB | 15.88 KB   |
| pk_aggregation               | 48950   | 151.72K | 837.49 KB    | 259.79 KB | 3.59 KB | 15.88 KB   |
| pk_generation                | 27737   | 57.82K  | 501.25 KB    | 348.73 KB | 3.59 KB | 15.88 KB   |
| share_decryption             | 22486   | 75.12K  | 466.54 KB    | 498.22 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption_ct0     | 27573   | 53.73K  | 500.84 KB    | 358.40 KB | 3.59 KB | 15.88 KB   |
| user_data_encryption_ct1     | 21399   | 46.27K  | 421.51 KB    | 325.19 KB | 3.59 KB | 15.88 KB   |

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
| **Compilation**      | 0.34 s    |
| **Execution**        | 0.49 s    |
| **VK Generation**    | 0.38 s    |
| **Proof Generation** | 0.89 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "75649"   |
| **Total Gates**      | "198355"  |
| **Circuit Size**     | 1.16 MB   |
| **Witness Size**     | 222.06 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk

| Metric               | Value    |
| -------------------- | -------- |
| **Compilation**      | 0.30 s   |
| **Execution**        | 0.31 s   |
| **VK Generation**    | 0.05 s   |
| **Proof Generation** | 0.12 s   |
| **Verification**     | 0.02 s   |
| **ACIR Opcodes**     | "344"    |
| **Total Gates**      | "6847"   |
| **Circuit Size**     | 90.54 KB |
| **Witness Size**     | 29.07 KB |
| **VK Size**          | 3.59 KB  |
| **Proof Size**       | 15.88 KB |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.20 s    |
| **Proof Generation** | 0.51 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "28577"   |
| **Total Gates**      | "92515"   |
| **Circuit Size**     | 538.83 KB |
| **Witness Size**     | 151.95 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_encryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.33 s    |
| **Execution**        | 0.48 s    |
| **VK Generation**    | 0.25 s    |
| **Proof Generation** | 0.60 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "47758"   |
| **Total Gates**      | "127047"  |
| **Circuit Size**     | 802.98 KB |
| **Witness Size**     | 512.11 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### sk_share_computation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.33 s    |
| **Execution**        | 0.45 s    |
| **VK Generation**    | 0.28 s    |
| **Proof Generation** | 0.81 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "56018"   |
| **Total Gates**      | "142625"  |
| **Circuit Size**     | 936.72 KB |
| **Witness Size**     | 165.20 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

### Threshold

#### decrypted_shares_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.33 s    |
| **Execution**        | 0.58 s    |
| **VK Generation**    | 0.23 s    |
| **Proof Generation** | 0.51 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "41673"   |
| **Total Gates**      | "104273"  |
| **Circuit Size**     | 1.12 MB   |
| **Witness Size**     | 111.14 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_aggregation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.34 s    |
| **Execution**        | 0.45 s    |
| **VK Generation**    | 0.30 s    |
| **Proof Generation** | 0.83 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "48950"   |
| **Total Gates**      | "151717"  |
| **Circuit Size**     | 837.49 KB |
| **Witness Size**     | 259.79 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### pk_generation

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.40 s    |
| **VK Generation**    | 0.14 s    |
| **Proof Generation** | 0.35 s    |
| **Verification**     | 0.02 s    |
| **ACIR Opcodes**     | "27737"   |
| **Total Gates**      | "57818"   |
| **Circuit Size**     | 501.25 KB |
| **Witness Size**     | 348.73 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### share_decryption

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.43 s    |
| **VK Generation**    | 0.17 s    |
| **Proof Generation** | 0.52 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "22486"   |
| **Total Gates**      | "75125"   |
| **Circuit Size**     | 466.54 KB |
| **Witness Size**     | 498.22 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption_ct0

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.41 s    |
| **VK Generation**    | 0.13 s    |
| **Proof Generation** | 0.33 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "27573"   |
| **Total Gates**      | "53732"   |
| **Circuit Size**     | 500.84 KB |
| **Witness Size**     | 358.40 KB |
| **VK Size**          | 3.59 KB   |
| **Proof Size**       | 15.88 KB  |

#### user_data_encryption_ct1

| Metric               | Value     |
| -------------------- | --------- |
| **Compilation**      | 0.32 s    |
| **Execution**        | 0.39 s    |
| **VK Generation**    | 0.12 s    |
| **Proof Generation** | 0.32 s    |
| **Verification**     | 0.03 s    |
| **ACIR Opcodes**     | "21399"   |
| **Total Gates**      | "46270"   |
| **Circuit Size**     | 421.51 KB |
| **Witness Size**     | 325.19 KB |
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

- **Nargo Version:** nargo version = 1.0.0-beta.16 noirc version =
  1.0.0-beta.16+2d46fca7203545cbbfb31a0d0328de6c10a8db95 (git version hash:
  2d46fca7203545cbbfb31a0d0328de6c10a8db95, is dirty: false)
- **Barretenberg Version:** 3.0.0-nightly.20260102
