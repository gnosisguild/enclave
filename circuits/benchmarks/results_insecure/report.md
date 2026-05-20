# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-18 13:44:31 UTC

**Git Branch:** `feat/1524`  
**Git Commit:** `7df3cad298ea4d0194af1dcea8afc397a7c0540e`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 26.98            | 15.88           |
| C1                   | 57818       | 0.33           | 25.28            | 15.88           |
| C2a                  | 142625      | 0.77           | 25.29            | 15.88           |
| C2b                  | 198355      | 0.83           | 25.44            | 15.88           |
| C3a                  | 132633      | 0.79           | 26.15            | 15.88           |
| C3b                  | 132633      | 0.79           | 26.15            | 15.88           |
| C4a                  | 92515       | 0.49           | 25.59            | 15.88           |
| C4b                  | 92515       | 0.49           | 25.59            | 15.88           |
| C5                   | 151717      | 0.79           | 25.38            | 15.88           |
| user_data_encryption | 53732       | 0.32           | 24.95            | 15.88           |
| C6                   | 86927       | 0.50           | 24.76            | 15.88           |
| C7                   | 104273      | 0.48           | 26.30            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.41 KB           | 3037910    | 175424       | 3213334   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972965    | 170200       | 3143165   |
| Π_dec    | 10.69 KB   | 3.41 KB           | 3549222    | 186764       | 3735986   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 304.50 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.79 s     | 10.69 KB   | 11.09 KB  |
| User            | P3    | per user input                   | 0.64 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.50 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 79.27 s    | 10.69 KB   | 14.09 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.04         |
| Committee Setup Completed                   | 20.24        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 304.50       |
| E3Request -> PublicKeyAggregated            | 307.02       |
| Application CT Gen                          | 0.32         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 79.27        |
| Entire Test                                 | 409.92       |

### Thread pool (same process as integration test)

| Setting                      | Value |
| ---------------------------- | ----- |
| Rayon threads                | 13    |
| Max simultaneous Rayon tasks | 1     |
| Cores available              | 14    |

### CPU-bound operation timings (tracked in-process)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.12    | 3    | 0.35      |
| CalculateDecryptionShare      | 0.61    | 3    | 1.83      |
| CalculateThresholdDecryption  | 0.58    | 1    | 0.58      |
| GenEsiSss                     | 0.12    | 3    | 0.37      |
| GenPkShareAndSkSss            | 0.22    | 3    | 0.67      |
| ZkDecryptedSharesAggregation  | 8.57    | 1    | 8.57      |
| ZkDecryptionAggregation       | 49.05   | 1    | 49.05     |
| ZkDkgAggregation              | 20.15   | 1    | 20.15     |
| ZkDkgShareDecryption          | 1.50    | 6    | 9.03      |
| ZkNodeDkgFold                 | 62.89   | 3    | 188.67    |
| ZkPkAggregation               | 2.16    | 1    | 2.16      |
| ZkPkBfv                       | 0.33    | 3    | 0.99      |
| ZkPkGeneration                | 1.33    | 3    | 3.99      |
| ZkShareComputation            | 2.69    | 6    | 16.16     |
| ZkShareEncryption             | 2.49    | 24   | 59.76     |
| ZkThresholdShareDecryption    | 6.05    | 3    | 18.14     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.29      |
| ZkVerifyShareProofs           | 0.23    | 5    | 1.13      |

Sum of tracked operation wall time: **381.88 s** (often much larger than end-to-end wall clock
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
