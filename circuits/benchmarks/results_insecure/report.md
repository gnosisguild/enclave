# Enclave ZK Circuit Benchmarks

**Generated:** 2026-04-29 10:02:54 UTC

**Git Branch:** `feat/benches`  
**Git Commit:** `36c01c62b86e2527279842280337f2f4724d2487`

**Committee Size:** `H=3`, `N=3`, `T=1`

---

## Protocol Summary

### Circuit Benchmarks

| Circuit              | Constraints | Prove time (s) | Verify time (ms) | Proof size (KB) |
| -------------------- | ----------- | -------------- | ---------------- | --------------- |
| C0                   | 6847        | 0.12           | 24.97            | 15.88           |
| C1                   | 57818       | 0.34           | 25.84            | 15.88           |
| C2a                  | 142625      | 0.78           | 25.50            | 15.88           |
| C2b                  | 198355      | 0.85           | 26.06            | 15.88           |
| C3a                  | 132633      | 0.78           | 25.14            | 15.88           |
| C3b                  | 132633      | 0.78           | 25.14            | 15.88           |
| C4a                  | 92515       | 0.50           | 25.00            | 15.88           |
| C4b                  | 92515       | 0.50           | 25.00            | 15.88           |
| C5                   | 151717      | 0.79           | 26.28            | 15.88           |
| user_data_encryption | 53732       | 0.33           | 24.58            | 15.88           |
| C6                   | 86927       | 0.52           | 25.34            | 15.88           |
| C7                   | 104273      | 0.49           | 25.42            | 15.88           |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.41 KB           | 3037922    | 175616       | 3213538   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2973073    | 170272       | 3143345   |
| Π_dec    | 10.69 KB   | 3.41 KB           | 3549077    | 186764       | 3735841   |

### Role / Phase / Activity

| Role            | Phase | Activity                         | Prove time | Proof size | Bandwidth |
| --------------- | ----- | -------------------------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation       | 379.38 s   | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | combine folds + C5               | 0.79 s     | 10.69 KB   | 11.09 KB  |
| User            | P3    | per user input                   | 0.65 s     | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)      | 0.52 s     | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | per computation output (C7+fold) | 80.31 s    | 10.69 KB   | 14.09 KB  |

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (wall clock)

| Phase                                       | Duration (s) |
| ------------------------------------------- | ------------ |
| Starting trbfv actor test                   | 0.00         |
| Setup completed                             | 2.98         |
| Committee Setup Completed                   | 20.25        |
| Committee Finalization Complete             | 0.01         |
| ThresholdShares -> PublicKeyAggregated      | 379.38       |
| E3Request -> PublicKeyAggregated            | 381.92       |
| Application CT Gen                          | 0.31         |
| Running FHE Application                     | 0.00         |
| Ciphertext published -> PlaintextAggregated | 80.31        |
| Entire Test                                 | 485.79       |

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
| CalculateDecryptionShare      | 0.61    | 3    | 1.84      |
| CalculateThresholdDecryption  | 0.57    | 1    | 0.57      |
| GenEsiSss                     | 0.12    | 3    | 0.37      |
| GenPkShareAndSkSss            | 0.22    | 3    | 0.67      |
| ZkDecryptedSharesAggregation  | 8.54    | 1    | 8.54      |
| ZkDecryptionAggregation       | 49.65   | 1    | 49.65     |
| ZkDkgAggregation              | 20.66   | 1    | 20.66     |
| ZkDkgShareDecryption          | 1.45    | 6    | 8.68      |
| ZkNodeDkgFold                 | 78.08   | 3    | 234.23    |
| ZkPkAggregation               | 2.19    | 1    | 2.19      |
| ZkPkBfv                       | 0.33    | 3    | 0.99      |
| ZkPkGeneration                | 1.34    | 3    | 4.01      |
| ZkShareComputation            | 2.65    | 6    | 15.91     |
| ZkShareEncryption             | 2.47    | 36   | 89.00     |
| ZkThresholdShareDecryption    | 6.20    | 3    | 18.59     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.30      |
| ZkVerifyShareProofs           | 0.22    | 5    | 1.08      |

Sum of tracked operation wall time: **457.62 s** (often much larger than end-to-end wall clock
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
