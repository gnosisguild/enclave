# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-21 10:09:51 UTC

**Git Branch:** `feat/1549`  
**Git Commit:** `1934c17dbe9cf5344fa0cb9bf247f5721a6d2f70`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 25.67            | 15.88           |
| C1                   | 57818       | 0.34           | 25.70            | 15.88           |
| C2a                  | 142625      | 0.78           | 26.08            | 15.88           |
| C2b                  | 198355      | 0.86           | 26.92            | 15.88           |
| C3a                  | 132633      | 0.80           | 26.97            | 15.88           |
| C3b                  | 132633      | 0.80           | 26.97            | 15.88           |
| C4a                  | 92515       | 0.50           | 26.99            | 15.88           |
| C4b                  | 92515       | 0.50           | 26.99            | 15.88           |
| C5                   | 151717      | 0.81           | 28.04            | 15.88           |
| user_data_encryption | 53732       | 0.34           | 26.75            | 15.88           |
| C6                   | 86927       | 0.52           | 26.36            | 15.88           |
| C7                   | 104273      | 0.50           | 26.75            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042552    | 176232       | 3218784   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2973037    | 170308       | 3143345   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553605    | 187212       | 3740817   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 305.20 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.81 s     | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 0.66 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.52 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 79.92 s    | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.05         |
| Committee Setup Completed                   | 20.25        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 305.20       |
| E3Request -> PublicKeyAggregated            | 307.80       |
| Application CT Gen                          | 0.31         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 79.92        |
| Entire Test                                 | 411.34       |

### Thread pool (same process as integration test)

| Setting                      | Value |
| ---------------------------- | ----- |
| Rayon threads                | 13    |
| Max simultaneous Rayon tasks | 1     |
| Cores available              | 14    |

### CPU-bound operation timings (tracked in-process)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.11    | 3    | 0.34      |
| CalculateDecryptionShare      | 0.61    | 3    | 1.82      |
| CalculateThresholdDecryption  | 0.56    | 1    | 0.56      |
| GenEsiSss                     | 0.12    | 3    | 0.37      |
| GenPkShareAndSkSss            | 0.23    | 3    | 0.68      |
| ZkDecryptedSharesAggregation  | 8.43    | 1    | 8.43      |
| ZkDecryptionAggregation       | 49.59   | 1    | 49.59     |
| ZkDkgAggregation              | 20.54   | 1    | 20.54     |
| ZkDkgShareDecryption          | 1.49    | 6    | 8.94      |
| ZkNodeDkgFold                 | 62.38   | 3    | 187.14    |
| ZkPkAggregation               | 2.13    | 1    | 2.13      |
| ZkPkBfv                       | 0.34    | 3    | 1.02      |
| ZkPkGeneration                | 1.38    | 3    | 4.14      |
| ZkShareComputation            | 2.76    | 6    | 16.54     |
| ZkShareEncryption             | 2.55    | 24   | 61.13     |
| ZkThresholdShareDecryption    | 6.14    | 3    | 18.42     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.29      |
| ZkVerifyShareProofs           | 0.23    | 5    | 1.14      |

Sum of tracked operation wall time: **383.24 s** (often much larger than end-to-end wall clock
because work runs in parallel).

## Raw circuit benchmark JSON (Nargo)

Source files for the **Circuit Benchmarks** table. Persist this directory with
`crisp_verify_gas.json` (and optional `integration_summary.json`) to regenerate the report without
re-running the integration test.

| File                                                  |
| ----------------------------------------------------- |
| `dkg_e_sm_share_computation_default.json`             |
| `dkg_pk_default.json`                                 |
| `dkg_share_decryption_default.json`                   |
| `dkg_share_encryption_default.json`                   |
| `dkg_sk_share_computation_default.json`               |
| `threshold_decrypted_shares_aggregation_default.json` |
| `threshold_pk_aggregation_default.json`               |
| `threshold_pk_generation_default.json`                |
| `threshold_share_decryption_default.json`             |
| `threshold_user_data_encryption_ct0_default.json`     |
| `threshold_user_data_encryption_ct1_default.json`     |

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

## Notes

- All nodes are executed on the same machine in this benchmark run, so inter-node network latency is
  effectively 0.
