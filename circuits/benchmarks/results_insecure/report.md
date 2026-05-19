# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-19 15:02:16 UTC

**Git Branch:** `feat/1525`  
**Git Commit:** `a9da36063d769bd6a7fb70c313315091e702f273`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 25.42            | 15.88           |
| C1                   | 57818       | 0.35           | 26.35            | 15.88           |
| C2a                  | 142625      | 0.78           | 26.20            | 15.88           |
| C2b                  | 198355      | 0.92           | 26.92            | 15.88           |
| C3a                  | 132633      | 0.79           | 27.00            | 15.88           |
| C3b                  | 132633      | 0.79           | 27.00            | 15.88           |
| C4a                  | 92515       | 0.50           | 26.01            | 15.88           |
| C4b                  | 92515       | 0.50           | 26.01            | 15.88           |
| C5                   | 151717      | 0.83           | 27.40            | 15.88           |
| user_data_encryption | 53732       | 0.37           | 28.29            | 15.88           |
| C6                   | 86927       | 0.52           | 27.28            | 15.88           |
| C7                   | 104273      | 0.50           | 26.38            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042712    | 176196       | 3218908   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972857    | 170200       | 3143057   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553819    | 187284       | 3741103   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 299.47 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.83 s     | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 0.70 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.52 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 78.56 s    | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.07         |
| Committee Setup Completed                   | 20.21        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 299.47       |
| E3Request -> PublicKeyAggregated            | 302.03       |
| Application CT Gen                          | 0.31         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 78.56        |
| Entire Test                                 | 404.20       |

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
| GenEsiSss                     | 0.13    | 3    | 0.38      |
| GenPkShareAndSkSss            | 0.23    | 3    | 0.68      |
| ZkDecryptedSharesAggregation  | 8.38    | 1    | 8.38      |
| ZkDecryptionAggregation       | 48.47   | 1    | 48.47     |
| ZkDkgAggregation              | 20.05   | 1    | 20.05     |
| ZkDkgShareDecryption          | 1.48    | 6    | 8.90      |
| ZkNodeDkgFold                 | 61.02   | 3    | 183.06    |
| ZkPkAggregation               | 2.12    | 1    | 2.12      |
| ZkPkBfv                       | 0.34    | 3    | 1.01      |
| ZkPkGeneration                | 1.36    | 3    | 4.09      |
| ZkShareComputation            | 2.71    | 6    | 16.29     |
| ZkShareEncryption             | 2.52    | 24   | 60.41     |
| ZkThresholdShareDecryption    | 6.08    | 3    | 18.25     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.29      |
| ZkVerifyShareProofs           | 0.21    | 5    | 1.07      |

Sum of tracked operation wall time: **376.16 s** (often much larger than end-to-end wall clock
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
