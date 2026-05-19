# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-19 14:41:50 UTC

**Git Branch:** `feat/1525`  
**Git Commit:** `d49dd7569dc6968edf92a66bd3ff48b6c5ee0503`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.13           | 27.23            | 15.88           |
| C1                   | 57818       | 0.35           | 26.50            | 15.88           |
| C2a                  | 142625      | 0.83           | 26.50            | 15.88           |
| C2b                  | 198355      | 0.90           | 27.09            | 15.88           |
| C3a                  | 132633      | 0.84           | 26.75            | 15.88           |
| C3b                  | 132633      | 0.84           | 26.75            | 15.88           |
| C4a                  | 92515       | 0.52           | 26.62            | 15.88           |
| C4b                  | 92515       | 0.52           | 26.62            | 15.88           |
| C5                   | 151717      | 0.83           | 27.39            | 15.88           |
| user_data_encryption | 53732       | 0.34           | 26.43            | 15.88           |
| C6                   | 86927       | 0.55           | 27.35            | 15.88           |
| C7                   | 104273      | 0.52           | 26.86            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042639    | 176112       | 3218751   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972941    | 170404       | 3143345   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553795    | 187260       | 3741055   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 314.97 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.83 s     | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 0.67 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.55 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 81.66 s    | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.08         |
| Committee Setup Completed                   | 20.22        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 314.97       |
| E3Request -> PublicKeyAggregated            | 317.62       |
| Application CT Gen                          | 0.32         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 81.66        |
| Entire Test                                 | 422.92       |

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
| ZkDecryptedSharesAggregation  | 8.51    | 1    | 8.51      |
| ZkDecryptionAggregation       | 51.14   | 1    | 51.14     |
| ZkDkgAggregation              | 21.24   | 1    | 21.24     |
| ZkDkgShareDecryption          | 1.51    | 6    | 9.07      |
| ZkNodeDkgFold                 | 64.91   | 3    | 194.73    |
| ZkPkAggregation               | 2.19    | 1    | 2.19      |
| ZkPkBfv                       | 0.35    | 3    | 1.04      |
| ZkPkGeneration                | 1.39    | 3    | 4.17      |
| ZkShareComputation            | 2.77    | 6    | 16.65     |
| ZkShareEncryption             | 2.59    | 24   | 62.19     |
| ZkThresholdShareDecryption    | 6.17    | 3    | 18.50     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.29      |
| ZkVerifyShareProofs           | 0.23    | 5    | 1.17      |

Sum of tracked operation wall time: **394.71 s** (often much larger than end-to-end wall clock
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
