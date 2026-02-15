# Enclave ZK Circuit Benchmarks

**Generated:** 2026-02-13 12:59:43 UTC

**Git Branch:** `ref/circuits-comp`  
**Git Commit:** `2403461592f5c628cb22b493586995f79bb698e1`

---

## Summary

### DKG

#### Timing Metrics

| Circuit | Compile | Execute | Prove | Verify |
|---------|---------|---------|-------|--------|
| e_sm_share_computation | 0.31 s | 0.52 s | 1.55 s | 0.02 s |
| pk | 0.25 s | 0.27 s | 0.12 s | 0.03 s |
| share_decryption | 0.25 s | 0.29 s | 0.23 s | 0.02 s |
| share_encryption | 0.29 s | 0.44 s | 0.61 s | 0.03 s |
| sk_share_computation | 0.30 s | 0.51 s | 1.54 s | 0.03 s |

#### Size & Circuit Metrics

| Circuit | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
|---------|---------|-------|--------------|---------|---------|------------|
| e_sm_share_computation | 90956 | 328.74K | 1.39 MB | 477.72 KB | 3.59 KB | 15.88 KB |
| pk | 344 | 6.85K | 87.84 KB | 29.07 KB | 3.59 KB | 15.88 KB |
| share_decryption | 3093 | 28.72K | 158.27 KB | 148.91 KB | 3.59 KB | 15.88 KB |
| share_encryption | 47758 | 127.69K | 798.14 KB | 512.11 KB | 3.59 KB | 15.88 KB |
| sk_share_computation | 90827 | 326.14K | 1.38 MB | 463.68 KB | 3.59 KB | 15.88 KB |

### Threshold

#### Timing Metrics

| Circuit | Compile | Execute | Prove | Verify |
|---------|---------|---------|-------|--------|
| decrypted_shares_aggregation_mod | 0.27 s | 0.33 s | 0.46 s | 0.02 s |
| pk_aggregation | 0.29 s | 0.45 s | 0.87 s | 0.02 s |
| pk_generation | 0.28 s | 0.38 s | 0.48 s | 0.03 s |
| share_decryption | 0.28 s | 0.40 s | 0.53 s | 0.03 s |
| user_data_encryption | 0.30 s | 0.47 s | 0.58 s | 0.02 s |

#### Size & Circuit Metrics

| Circuit | Opcodes | Gates | Circuit Size | Witness | VK Size | Proof Size |
|---------|---------|-------|--------------|---------|---------|------------|
| decrypted_shares_aggregation_mod | 31544 | 80.74K | 509.84 KB | 77.56 KB | 3.59 KB | 15.88 KB |
| pk_aggregation | 47817 | 169.89K | 884.11 KB | 360.86 KB | 3.59 KB | 15.88 KB |
| pk_generation | 30019 | 65.61K | 542.16 KB | 447.05 KB | 3.59 KB | 15.88 KB |
| share_decryption | 30570 | 85.48K | 541.56 KB | 522.86 KB | 3.59 KB | 15.88 KB |
| user_data_encryption | 56601 | 106.72K | 847.64 KB | 691.13 KB | 3.59 KB | 15.88 KB |

## Circuit Details

### DKG

#### e_sm_share_computation

| Metric | Value |
|--------|-------|
| **Compilation** | 0.31 s |
| **Execution** | 0.52 s |
| **VK Generation** | 0.59 s |
| **Proof Generation** | 1.55 s |
| **Verification** | 0.02 s |
| **ACIR Opcodes** | "90956" |
| **Total Gates** | "328743" |
| **Circuit Size** | 1.39 MB |
| **Witness Size** | 477.72 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### pk

| Metric | Value |
|--------|-------|
| **Compilation** | 0.25 s |
| **Execution** | 0.27 s |
| **VK Generation** | 0.05 s |
| **Proof Generation** | 0.12 s |
| **Verification** | 0.03 s |
| **ACIR Opcodes** | "344" |
| **Total Gates** | "6846" |
| **Circuit Size** | 87.84 KB |
| **Witness Size** | 29.07 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### share_decryption

| Metric | Value |
|--------|-------|
| **Compilation** | 0.25 s |
| **Execution** | 0.29 s |
| **VK Generation** | 0.09 s |
| **Proof Generation** | 0.23 s |
| **Verification** | 0.02 s |
| **ACIR Opcodes** | "3093" |
| **Total Gates** | "28720" |
| **Circuit Size** | 158.27 KB |
| **Witness Size** | 148.91 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### share_encryption

| Metric | Value |
|--------|-------|
| **Compilation** | 0.29 s |
| **Execution** | 0.44 s |
| **VK Generation** | 0.26 s |
| **Proof Generation** | 0.61 s |
| **Verification** | 0.03 s |
| **ACIR Opcodes** | "47758" |
| **Total Gates** | "127691" |
| **Circuit Size** | 798.14 KB |
| **Witness Size** | 512.11 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### sk_share_computation

| Metric | Value |
|--------|-------|
| **Compilation** | 0.30 s |
| **Execution** | 0.51 s |
| **VK Generation** | 0.66 s |
| **Proof Generation** | 1.54 s |
| **Verification** | 0.03 s |
| **ACIR Opcodes** | "90827" |
| **Total Gates** | "326138" |
| **Circuit Size** | 1.38 MB |
| **Witness Size** | 463.68 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |


### Threshold

#### decrypted_shares_aggregation_mod

| Metric | Value |
|--------|-------|
| **Compilation** | 0.27 s |
| **Execution** | 0.33 s |
| **VK Generation** | 0.18 s |
| **Proof Generation** | 0.46 s |
| **Verification** | 0.02 s |
| **ACIR Opcodes** | "31544" |
| **Total Gates** | "80740" |
| **Circuit Size** | 509.84 KB |
| **Witness Size** | 77.56 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### pk_aggregation

| Metric | Value |
|--------|-------|
| **Compilation** | 0.29 s |
| **Execution** | 0.45 s |
| **VK Generation** | 0.35 s |
| **Proof Generation** | 0.87 s |
| **Verification** | 0.02 s |
| **ACIR Opcodes** | "47817" |
| **Total Gates** | "169890" |
| **Circuit Size** | 884.11 KB |
| **Witness Size** | 360.86 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### pk_generation

| Metric | Value |
|--------|-------|
| **Compilation** | 0.28 s |
| **Execution** | 0.38 s |
| **VK Generation** | 0.16 s |
| **Proof Generation** | 0.48 s |
| **Verification** | 0.03 s |
| **ACIR Opcodes** | "30019" |
| **Total Gates** | "65606" |
| **Circuit Size** | 542.16 KB |
| **Witness Size** | 447.05 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### share_decryption

| Metric | Value |
|--------|-------|
| **Compilation** | 0.28 s |
| **Execution** | 0.40 s |
| **VK Generation** | 0.19 s |
| **Proof Generation** | 0.53 s |
| **Verification** | 0.03 s |
| **ACIR Opcodes** | "30570" |
| **Total Gates** | "85478" |
| **Circuit Size** | 541.56 KB |
| **Witness Size** | 522.86 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |

#### user_data_encryption

| Metric | Value |
|--------|-------|
| **Compilation** | 0.30 s |
| **Execution** | 0.47 s |
| **VK Generation** | 0.23 s |
| **Proof Generation** | 0.58 s |
| **Verification** | 0.02 s |
| **ACIR Opcodes** | "56601" |
| **Total Gates** | "106725" |
| **Circuit Size** | 847.64 KB |
| **Witness Size** | 691.13 KB |
| **VK Size** | 3.59 KB |
| **Proof Size** | 15.88 KB |


## System Information

### Hardware

- **CPU:** Apple M4 Pro
- **CPU Cores:** 14
- **RAM:** 48.00 GB
- **OS:** Darwin
- **Architecture:** arm64

### Software

- **Nargo Version:** nargo version = 1.0.0-beta.15 noirc version = 1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663 (git version hash: 83245db91dcf63420ef4bcbbd85b98f397fee663, is dirty: false) 
- **Barretenberg Version:** 3.0.0-nightly.20251104 

