# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-29 13:43:47 UTC

**Git Branch:** `feat/benches`  
**Git Commit:** `36c01c62b86e2527279842280337f2f4724d2487`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 287764      | 1.46           | 25.10            | 15.88           |
| C1                   | 2432074     | 9.37           | 27.22            | 15.88           |
| C2a                  | 3879330     | 10.95          | 26.13            | 15.88           |
| C2b                  | 5739750     | 19.54          | 26.31            | 15.88           |
| C3a                  | 3764144     | 11.16          | 27.82            | 15.88           |
| C3b                  | 3764144     | 11.16          | 27.82            | 15.88           |
| C4a                  | 2564001     | 9.30           | 27.21            | 15.88           |
| C4b                  | 2564001     | 9.30           | 27.21            | 15.88           |
| C5                   | 4395328     | 17.68          | 27.28            | 15.88           |
| user_data_encryption | 1678200     | 5.81           | 25.70            | 15.88           |
| C6                   | 3001847     | 10.47          | 27.29            | 15.88           |
| C7                   | 128310      | 0.53           | 27.59            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.41 KB           | 3037922    | 175556       | 3213538   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972869    | 193468       | 3166337   |
| Π_dec    | 10.69 KB   | 3.41 KB           | 3549077    | 186764       | 3735841   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 7204.02 s  | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | combine folds + C5               | 17.68 s    | 10.69 KB   | 11.09 KB  |
| User            | P3    | per user input                   | 11.23 s    | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 10.47 s    | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 814.17 s   | 10.69 KB   | 14.09 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 3.27         |
| Committee Setup Completed                   | 20.26        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 7204.02      |
| E3Request -> PublicKeyAggregated            | 7211.07      |
| Application CT Gen                          | 7.75         |
| Running FHE Application                     | 0.09         |
| Ciphertext published -> PlaintextAggregated | 814.17       |
| Entire Test                                 | 8056.62      |

### Thread pool (same process as integration test)

| Setting                      | Value |
| ---------------------------- | ----- |
| Rayon threads                | 13    |
| Max simultaneous Rayon tasks | 1     |
| Cores available              | 14    |

### CPU-bound operation timings (tracked in-process)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.60    | 3    | 1.80      |
| CalculateDecryptionShare      | 2.12    | 3    | 6.37      |
| CalculateThresholdDecryption  | 1.94    | 1    | 1.94      |
| GenEsiSss                     | 0.76    | 3    | 2.27      |
| GenPkShareAndSkSss            | 1.23    | 3    | 3.69      |
| ZkDecryptedSharesAggregation  | 18.90   | 1    | 18.90     |
| ZkDecryptionAggregation       | 48.06   | 1    | 48.06     |
| ZkDkgAggregation              | 20.90   | 1    | 20.90     |
| ZkDkgShareDecryption          | 30.16   | 6    | 180.96    |
| ZkNodeDkgFold                 | 102.34  | 3    | 307.01    |
| ZkPkAggregation               | 49.02   | 1    | 49.02     |
| ZkPkBfv                       | 3.84    | 3    | 11.52     |
| ZkPkGeneration                | 65.13   | 3    | 195.40    |
| ZkShareComputation            | 52.43   | 6    | 314.56    |
| ZkShareEncryption             | 112.96  | 54   | 6100.05   |
| ZkThresholdShareDecryption    | 244.37  | 3    | 733.10    |
| ZkVerifyShareDecryptionProofs | 0.09    | 3    | 0.28      |
| ZkVerifyShareProofs           | 0.27    | 5    | 1.33      |

Sum of tracked operation wall time: **7997.16 s** (often much larger than end-to-end wall clock
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
