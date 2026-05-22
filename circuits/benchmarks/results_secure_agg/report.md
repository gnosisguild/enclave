# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-22 17:39:45 UTC

**Git Branch:** `feat/1549`  
**Git Commit:** `f5c2fef8490fc34fe7357743220321af9626c879`

**Committee Size:** `H=3`, `N=3`, `T=1`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `secure`                                     |
| BFV preset (artifacts)                                | `secure-8192`                                |
| BFV preset (enum)                                     | `SecureThreshold8192`                        |
| λ (smudging / error)                                  | 60                                           |
| Nodes spawned (builder)                               | 20                                           |
| Network model                                         | `in_process_bus`                             |
| Testmode harness                                      | true                                         |
| `proof_aggregation_enabled`                           | true                                         |
| `BENCHMARK_MULTITHREAD_JOBS` (max concurrent ZK jobs) | 13                                           |
| Rayon worker threads                                  | 13                                           |
| CPU cores (host)                                      | 14                                           |
| `dkg_fold_attestation_verifier` (EIP-712)             | `0x7969c5eD335650692Bc04293B07F5BF2e7A673C0` |
| Verbose logging (`run_benchmarks.sh --verbose`)       | false                                        |

### Hardware & software (Nargo / Barretenberg host)

|                  |                                                                                                                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **CPU**          | Apple M4 Pro                                                                                                                                                                       |
| **CPU cores**    | 14                                                                                                                                                                                 |
| **RAM**          | 48.00 GB                                                                                                                                                                           |
| **OS**           | Darwin                                                                                                                                                                             |
| **Architecture** | arm64                                                                                                                                                                              |
| **Nargo**        | nargo version = 1.0.0-beta.16 noirc version = 1.0.0-beta.16+2d46fca7203545cbbfb31a0d0328de6c10a8db95 (git version hash: 2d46fca7203545cbbfb31a0d0328de6c10a8db95, is dirty: false) |
| **Barretenberg** | 3.0.0-nightly.20260102                                                                                                                                                             |

---

## Audit status

On-chain verify gas: **complete** (CRISP Π_user + Enclave Π_DKG / Π_dec replay).

---

## Measurement methodology

| Metric kind          | Source                                           | Meaning                                                                                    | Do **not** use for                                             |
| -------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| **wall_clock**       | `test_trbfv_actor` phase timers / HLC event span | End-to-end wait in the in-process test harness                                             | Production WAN latency; per-node deployment cost               |
| **isolated_nargo**   | `benchmark_circuit.sh` per circuit               | Single `bb prove` on oracle witness, one circuit at a time                                 | Full protocol pipeline (different witness path)                |
| **tracked_job_wall** | `MultithreadReport` per `ComputeRequest`         | Wall time of each job on the shared Rayon pool (≤ `BENCHMARK_MULTITHREAD_JOBS` concurrent) | End-to-end time — **sums exceed wall clock** when jobs overlap |

**Harness limits (integration):** all ciphernodes share one process and bus
(`network_model: in_process_bus`); sortition registers extra nodes; `testmode_*` enabled. Compare
runs only with the same `benchmark_mode`, proof-aggregation flag, `BENCHMARK_MULTITHREAD_JOBS`,
commit, and hardware.

---

## Protocol Summary

### Circuit Benchmarks (isolated Nargo + Barretenberg)

Single-circuit `bb prove` on the benchmark oracle witness (not the integration actor pipeline).

| Circuit              | Constraints | Prove (s) | Verify (ms) | Proof (KB) |
| -------------------- | ----------- | --------- | ----------- | ---------- |
| C0                   | 287764      | 1.47      | 24.77       | 15.88      |
| C1                   | 2432074     | 9.59      | 27.85       | 15.88      |
| C2a                  | 3879330     | 11.23     | 25.90       | 15.88      |
| C2b                  | 5739750     | 19.89     | 25.99       | 15.88      |
| C3a                  | 3764144     | 11.76     | 26.38       | 15.88      |
| C3b                  | 3764144     | 11.76     | 26.38       | 15.88      |
| C4a                  | 2564001     | 9.72      | 26.12       | 15.88      |
| C4b                  | 2564001     | 9.72      | 26.12       | 15.88      |
| C5                   | 4395328     | 18.36     | 26.08       | 15.88      |
| user_data_encryption | 1678200     | 6.39      | 29.08       | 15.88      |
| C6                   | 3001847     | 10.65     | 27.81       | 15.88      |
| C7                   | 128310      | 0.59      | 27.36       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3042585    | 176232       | 3218817   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972941    | 193372       | 3166313   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3553811    | 187392       | 3741203   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration  | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | --------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 1350.54 s | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 157.91 s  | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.94 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.65 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 389.34 s  | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 70.08 s   | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **126.59 s** —
not comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 3.25         |
| Committee Setup Completed                                          | `wall_clock` | 20.24        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 157.91       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 1350.54      |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 1357.85      |
| Application CT Gen                                                 | `wall_clock` | 7.89         |
| Running FHE Application                                            | `wall_clock` | 0.07         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 70.08        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 389.34       |
| Entire Test                                                        | `wall_clock` | 1778.65      |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.61    | 3    | 1.84      |
| CalculateDecryptionShare      | 2.18    | 3    | 6.55      |
| CalculateThresholdDecryption  | 1.94    | 1    | 1.94      |
| GenEsiSss                     | 0.80    | 3    | 2.41      |
| GenPkShareAndSkSss            | 1.47    | 3    | 4.41      |
| NodeDkgFold/c2ab_fold         | 7.19    | 3    | 21.58     |
| NodeDkgFold/c3a_fold          | 51.03   | 3    | 153.08    |
| NodeDkgFold/c3ab_fold         | 6.83    | 3    | 20.49     |
| NodeDkgFold/c3b_fold          | 55.39   | 3    | 166.16    |
| NodeDkgFold/c4ab_fold         | 8.46    | 3    | 25.37     |
| NodeDkgFold/node_fold         | 16.32   | 3    | 48.95     |
| ZkDecryptedSharesAggregation  | 19.08   | 1    | 19.08     |
| ZkDecryptionAggregation       | 50.56   | 1    | 50.56     |
| ZkDkgAggregation              | 21.03   | 1    | 21.03     |
| ZkDkgShareDecryption          | 52.18   | 6    | 313.05    |
| ZkNodeDkgFold                 | 145.22  | 3    | 435.65    |
| ZkPkAggregation               | 105.56  | 1    | 105.56    |
| ZkPkBfv                       | 5.97    | 3    | 17.90     |
| ZkPkGeneration                | 379.49  | 3    | 1138.47   |
| ZkShareComputation            | 101.48  | 6    | 608.87    |
| ZkShareEncryption             | 288.67  | 36   | 10392.20  |
| ZkThresholdShareDecryption    | 305.78  | 3    | 917.33    |
| ZkVerifyShareDecryptionProofs | 0.11    | 3    | 0.34      |
| ZkVerifyShareProofs           | 0.34    | 5    | 1.70      |

Sum of tracked job wall time: **14474.52 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 7.19    | 3    | 21.58     |
| c3a_fold  | 51.03   | 3    | 153.08    |
| c3ab_fold | 6.83    | 3    | 20.49     |
| c3b_fold  | 55.39   | 3    | 166.16    |
| c4ab_fold | 8.46    | 3    | 25.37     |
| node_fold | 16.32   | 3    | 48.95     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 19.08   | 1    | 19.08     |
| ZkDecryptionAggregation      | 50.56   | 1    | 50.56     |
| ZkDkgAggregation             | 21.03   | 1    | 21.03     |
| ZkNodeDkgFold                | 145.22  | 3    | 435.65    |
| ZkPkAggregation              | 105.56  | 1    | 105.56    |

Sum of aggregation job tracked time: **631.88 s** (parallel CPU work; not P1/P2 wall clock).

### Folded on-chain artifacts (exported for Π_DKG / Π_dec gas)

| Artifact              | Proof (bytes) | Public inputs (bytes) |
| --------------------- | ------------- | --------------------- |
| dkg_aggregator        | 10944         | 480                   |
| decryption_aggregator | 10944         | 3552                  |

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

## Notes

- All nodes are executed on the same machine in this benchmark run, so inter-node network latency is
  effectively 0.
