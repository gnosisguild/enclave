# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-21 08:04:33 UTC

**Git Branch:** `feat/1525`  
**Git Commit:** `6e6e3d396b3ddc7b5e9901bb66c404cc58dc06c1`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 28.68            | 15.88           |
| C1                   | 57818       | 0.33           | 25.71            | 15.88           |
| C2a                  | 142625      | 0.75           | 24.48            | 15.88           |
| C2b                  | 198355      | 0.82           | 25.54            | 15.88           |
| C3a                  | 132633      | 0.80           | 25.02            | 15.88           |
| C3b                  | 132633      | 0.80           | 25.02            | 15.88           |
| C4a                  | 92515       | 0.48           | 24.45            | 15.88           |
| C4b                  | 92515       | 0.48           | 24.45            | 15.88           |
| C5                   | 151717      | 0.79           | 24.83            | 15.88           |
| user_data_encryption | 53732       | 0.34           | 24.75            | 15.88           |
| C6                   | 86927       | 0.50           | 24.94            | 15.88           |
| C7                   | 104273      | 0.47           | 25.49            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042369    | 176052       | 3218421   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972905    | 170392       | 3143297   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553763    | 187368       | 3741131   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 302.76 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.79 s     | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 0.66 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.50 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 79.62 s    | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.05         |
| Committee Setup Completed                   | 20.24        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 302.76       |
| E3Request -> PublicKeyAggregated            | 305.29       |
| Application CT Gen                          | 0.31         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 79.62        |
| Entire Test                                 | 408.53       |

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
| GenEsiSss                     | 0.12    | 3    | 0.37      |
| GenPkShareAndSkSss            | 0.22    | 3    | 0.67      |
| ZkDecryptedSharesAggregation  | 8.41    | 1    | 8.41      |
| ZkDecryptionAggregation       | 49.38   | 1    | 49.38     |
| ZkDkgAggregation              | 20.65   | 1    | 20.65     |
| ZkDkgShareDecryption          | 1.47    | 6    | 8.79      |
| ZkNodeDkgFold                 | 62.10   | 3    | 186.30    |
| ZkPkAggregation               | 2.15    | 1    | 2.15      |
| ZkPkBfv                       | 0.33    | 3    | 0.99      |
| ZkPkGeneration                | 1.36    | 3    | 4.07      |
| ZkShareComputation            | 2.68    | 6    | 16.08     |
| ZkShareEncryption             | 2.50    | 24   | 60.01     |
| ZkThresholdShareDecryption    | 6.12    | 3    | 18.37     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.31      |
| ZkVerifyShareProofs           | 0.22    | 5    | 1.08      |

Sum of tracked operation wall time: **380.33 s** (often much larger than end-to-end wall clock
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
