# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-29 08:49:00 UTC

**Git Branch:** `feat/benches`  
**Git Commit:** `875e491b919e75a3bfe5e9a1a0a30083b3778272`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 26.80            | 15.88           |
| C1                   | 57818       | 0.33           | 25.43            | 15.88           |
| C2a                  | 142625      | 0.76           | 25.39            | 15.88           |
| C2b                  | 198355      | 0.83           | 25.57            | 15.88           |
| C3a                  | 132633      | 0.79           | 26.04            | 15.88           |
| C3b                  | 132633      | 0.79           | 26.04            | 15.88           |
| C4a                  | 92515       | 0.49           | 26.02            | 15.88           |
| C4b                  | 92515       | 0.49           | 26.02            | 15.88           |
| C5                   | 151717      | 0.80           | 27.27            | 15.88           |
| user_data_encryption | 53732       | 0.32           | 26.01            | 15.88           |
| C6                   | 86927       | 0.51           | 25.42            | 15.88           |
| C7                   | 104273      | 0.49           | 26.02            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.41 KB           | 3037763    | 175460       | 3213223   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972989    | 170392       | 3143381   |
| Π_dec    | 10.69 KB   | 3.41 KB           | 3548967    | 186656       | 3735623   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 373.35 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.80 s     | 10.69 KB   | 11.09 KB  |
| User            | P3    | per user input                   | 0.63 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.51 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 80.05 s    | 10.69 KB   | 14.09 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.02         |
| Committee Setup Completed                   | 20.25        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 373.35       |
| E3Request -> PublicKeyAggregated            | 375.86       |
| Application CT Gen                          | 0.31         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 80.05        |
| Entire Test                                 | 479.51       |

### Thread pool (same process as integration test)

| Setting                      | Value |
| ---------------------------- | ----- |
| Rayon threads                | 13    |
| Max simultaneous Rayon tasks | 1     |
| Cores available              | 14    |

### CPU-bound operation timings (tracked in-process)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.11    | 3    | 0.33      |
| CalculateDecryptionShare      | 0.61    | 3    | 1.82      |
| CalculateThresholdDecryption  | 0.56    | 1    | 0.56      |
| GenEsiSss                     | 0.12    | 3    | 0.36      |
| GenPkShareAndSkSss            | 0.22    | 3    | 0.67      |
| ZkDecryptedSharesAggregation  | 8.66    | 1    | 8.66      |
| ZkDecryptionAggregation       | 49.79   | 1    | 49.79     |
| ZkDkgAggregation              | 19.82   | 1    | 19.82     |
| ZkDkgShareDecryption          | 1.43    | 6    | 8.59      |
| ZkNodeDkgFold                 | 76.58   | 3    | 229.73    |
| ZkPkAggregation               | 2.12    | 1    | 2.12      |
| ZkPkBfv                       | 0.33    | 3    | 0.99      |
| ZkPkGeneration                | 1.33    | 3    | 3.99      |
| ZkShareComputation            | 2.64    | 6    | 15.83     |
| ZkShareEncryption             | 2.46    | 36   | 88.61     |
| ZkThresholdShareDecryption    | 6.05    | 3    | 18.14     |
| ZkVerifyShareDecryptionProofs | 0.09    | 3    | 0.28      |
| ZkVerifyShareProofs           | 0.21    | 5    | 1.07      |

Sum of tracked operation wall time: **451.36 s** (often much larger than end-to-end wall clock
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
