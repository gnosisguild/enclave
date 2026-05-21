# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-20 09:19:34 UTC

**Git Branch:** `feat/1525`  
**Git Commit:** `e0d477e9e98185779250f4092947741e84169b02`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 287764      | 1.47           | 26.17            | 15.88           |
| C1                   | 2432074     | 9.49           | 26.53            | 15.88           |
| C2a                  | 3879330     | 10.88          | 25.37            | 15.88           |
| C2b                  | 5739750     | 19.44          | 25.98            | 15.88           |
| C3a                  | 3764144     | 11.33          | 26.23            | 15.88           |
| C3b                  | 3764144     | 11.33          | 26.23            | 15.88           |
| C4a                  | 2564001     | 9.30           | 26.50            | 15.88           |
| C4b                  | 2564001     | 9.30           | 26.50            | 15.88           |
| C5                   | 4395328     | 17.56          | 26.87            | 15.88           |
| user_data_encryption | 1678200     | 5.98           | 25.90            | 15.88           |
| C6                   | 3001847     | 10.47          | 27.66            | 15.88           |
| C7                   | 128310      | 0.54           | 26.29            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042688    | 176160       | 3218848   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972893    | 193336       | 3166229   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553795    | 187260       | 3741055   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 5158.13 s  | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | combine folds + C5               | 17.56 s    | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                   | 11.40 s    | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 10.47 s    | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 835.14 s   | 10.69 KB   | 14.16 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.27         |
| Committee Setup Completed                   | 20.28        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 5158.13      |
| E3Request -> PublicKeyAggregated            | 5165.21      |
| Application CT Gen                          | 7.71         |
| Running FHE Application                     | 0.07         |
| Ciphertext published -> PlaintextAggregated | 835.14       |
| Entire Test                                 | 6031.70      |

### Thread pool (same process as integration test)

| Setting                      | Value |
| ---------------------------- | ----- |
| Rayon threads                | 13    |
| Max simultaneous Rayon tasks | 1     |
| Cores available              | 14    |

### CPU-bound operation timings (tracked in-process)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.61    | 3    | 1.84      |
| CalculateDecryptionShare      | 2.12    | 3    | 6.36      |
| CalculateThresholdDecryption  | 1.96    | 1    | 1.96      |
| GenEsiSss                     | 0.76    | 3    | 2.27      |
| GenPkShareAndSkSss            | 1.24    | 3    | 3.73      |
| ZkDecryptedSharesAggregation  | 18.98   | 1    | 18.98     |
| ZkDecryptionAggregation       | 48.34   | 1    | 48.34     |
| ZkDkgAggregation              | 20.01   | 1    | 20.01     |
| ZkDkgShareDecryption          | 30.28   | 6    | 181.67    |
| ZkNodeDkgFold                 | 78.31   | 3    | 234.93    |
| ZkPkAggregation               | 49.05   | 1    | 49.05     |
| ZkPkBfv                       | 3.85    | 3    | 11.55     |
| ZkPkGeneration                | 66.06   | 3    | 198.17    |
| ZkShareComputation            | 52.53   | 6    | 315.20    |
| ZkShareEncryption             | 114.61  | 36   | 4125.90   |
| ZkThresholdShareDecryption    | 251.23  | 3    | 753.69    |
| ZkVerifyShareDecryptionProofs | 0.09    | 3    | 0.28      |
| ZkVerifyShareProofs           | 0.26    | 5    | 1.32      |

Sum of tracked operation wall time: **5975.27 s** (often much larger than end-to-end wall clock
because work runs in parallel).

## Raw circuit benchmark JSON (Nargo)

Source files for the **Circuit Benchmarks** table. Persist this directory with
`crisp_verify_gas.json` (and optional `integration_summary.json`) to regenerate the report without
re-running the integration test.

| File                                                  |
| ----------------------------------------------------- |
| `config_default.json`                                 |
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
