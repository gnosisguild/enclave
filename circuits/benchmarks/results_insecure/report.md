# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-19 14:14:09 UTC

**Git Branch:** `feat/1525`  
**Git Commit:** `cfe60a5290ceb27522bdee8a5e7d508e0e556d08`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.13           | 28.33            | 15.88           |
| C1                   | 57818       | 0.35           | 27.90            | 15.88           |
| C2a                  | 142625      | 0.83           | 29.66            | 15.88           |
| C2b                  | 198355      | 0.89           | 28.73            | 15.88           |
| C3a                  | 132633      | 0.83           | 28.25            | 15.88           |
| C3b                  | 132633      | 0.83           | 28.25            | 15.88           |
| C4a                  | 92515       | 0.52           | 29.91            | 15.88           |
| C4b                  | 92515       | 0.52           | 29.91            | 15.88           |
| C5                   | 151717      | 0.85           | 28.47            | 15.88           |
| user_data_encryption | 53732       | 0.35           | 27.79            | 15.88           |
| C6                   | 86927       | 0.54           | 28.57            | 15.88           |
| C7                   | 104273      | 0.52           | 28.75            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042773    | 176244       | 3219017   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972965    | 170452       | 3143417   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553807    | 187284       | 3741091   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 309.65 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.85 s     | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 0.68 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.54 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 82.61 s    | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.08         |
| Committee Setup Completed                   | 20.23        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 309.65       |
| E3Request -> PublicKeyAggregated            | 312.28       |
| Application CT Gen                          | 0.32         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 82.61        |
| Entire Test                                 | 418.55       |

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
| CalculateDecryptionShare      | 0.61    | 3    | 1.84      |
| CalculateThresholdDecryption  | 0.57    | 1    | 0.57      |
| GenEsiSss                     | 0.13    | 3    | 0.38      |
| GenPkShareAndSkSss            | 0.23    | 3    | 0.69      |
| ZkDecryptedSharesAggregation  | 8.66    | 1    | 8.66      |
| ZkDecryptionAggregation       | 51.72   | 1    | 51.72     |
| ZkDkgAggregation              | 20.93   | 1    | 20.93     |
| ZkDkgShareDecryption          | 1.50    | 6    | 8.98      |
| ZkNodeDkgFold                 | 63.68   | 3    | 191.04    |
| ZkPkAggregation               | 2.15    | 1    | 2.15      |
| ZkPkBfv                       | 0.35    | 3    | 1.04      |
| ZkPkGeneration                | 1.39    | 3    | 4.16      |
| ZkShareComputation            | 2.75    | 6    | 16.47     |
| ZkShareEncryption             | 2.55    | 24   | 61.20     |
| ZkThresholdShareDecryption    | 6.23    | 3    | 18.69     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.31      |
| ZkVerifyShareProofs           | 0.24    | 5    | 1.20      |

Sum of tracked operation wall time: **390.36 s** (often much larger than end-to-end wall clock
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
