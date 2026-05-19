# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-19 13:45:13 UTC

**Git Branch:** `feat/1525`  
**Git Commit:** `ab26bae432a591aa0345b8b7b64e069f28b26bc1`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 25.86            | 15.88           |
| C1                   | 57818       | 0.36           | 27.07            | 15.88           |
| C2a                  | 142625      | 0.82           | 27.33            | 15.88           |
| C2b                  | 198355      | 0.91           | 27.67            | 15.88           |
| C3a                  | 132633      | 0.85           | 27.21            | 15.88           |
| C3b                  | 132633      | 0.85           | 27.21            | 15.88           |
| C4a                  | 92515       | 0.53           | 27.01            | 15.88           |
| C4b                  | 92515       | 0.53           | 27.01            | 15.88           |
| C5                   | 151717      | 0.85           | 27.10            | 15.88           |
| user_data_encryption | 53732       | 0.34           | 26.27            | 15.88           |
| C6                   | 86927       | 0.54           | 28.16            | 15.88           |
| C7                   | 104273      | 0.50           | 27.64            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042828    | 176148       | 3218976   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972965    | 170272       | 3143237   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553758    | 187236       | 3740994   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 310.65 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.85 s     | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 0.67 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.54 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 82.11 s    | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.06         |
| Committee Setup Completed                   | 20.24        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 310.65       |
| E3Request -> PublicKeyAggregated            | 313.31       |
| Application CT Gen                          | 0.32         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 82.11        |
| Entire Test                                 | 419.06       |

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
| CalculateDecryptionShare      | 0.62    | 3    | 1.87      |
| CalculateThresholdDecryption  | 0.57    | 1    | 0.57      |
| GenEsiSss                     | 0.13    | 3    | 0.38      |
| GenPkShareAndSkSss            | 0.23    | 3    | 0.69      |
| ZkDecryptedSharesAggregation  | 8.60    | 1    | 8.60      |
| ZkDecryptionAggregation       | 51.19   | 1    | 51.19     |
| ZkDkgAggregation              | 21.08   | 1    | 21.08     |
| ZkDkgShareDecryption          | 1.50    | 6    | 9.00      |
| ZkNodeDkgFold                 | 63.75   | 3    | 191.24    |
| ZkPkAggregation               | 2.19    | 1    | 2.19      |
| ZkPkBfv                       | 0.34    | 3    | 1.03      |
| ZkPkGeneration                | 1.38    | 3    | 4.15      |
| ZkShareComputation            | 2.75    | 6    | 16.52     |
| ZkShareEncryption             | 2.57    | 24   | 61.79     |
| ZkThresholdShareDecryption    | 6.26    | 3    | 18.77     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.30      |
| ZkVerifyShareProofs           | 0.23    | 5    | 1.14      |

Sum of tracked operation wall time: **390.82 s** (often much larger than end-to-end wall clock
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
