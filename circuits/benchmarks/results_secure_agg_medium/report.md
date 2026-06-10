# Interfold ZK Circuit Benchmarks

**Generated:** 2026-06-07 16:36:17 UTC

**Git Branch:** `main`  
**Git Commit:** `f653c04bc6a1e6d18da0c37b23502e28a5f59648`

**Committee Size:** `H=8`, `N=10`, `T=4`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `secure`                                     |
| BFV preset (artifacts)                                | `secure-8192`                                |
| BFV preset (enum)                                     | `SecureThreshold8192`                        |
| λ (smudging / error)                                  | 50                                           |
| Nodes spawned (builder)                               | 18                                           |
| Network model                                         | `in_process_bus`                             |
| Testmode harness                                      | true                                         |
| `proof_aggregation_enabled`                           | true                                         |
| `BENCHMARK_MULTITHREAD_JOBS` (max concurrent ZK jobs) | 13                                           |
| Rayon worker threads                                  | 13                                           |
| CPU cores (host)                                      | 14                                           |
| `dkg_fold_attestation_verifier` (EIP-712)             | `0x7969c5eD335650692Bc04293B07F5BF2e7A673C0` |
| Verbose logging (`run_benchmarks.sh --verbose`)       | true                                         |

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

On-chain verify gas: **complete** (CRISP Π_user + Interfold Π_DKG / Π_dec replay).

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
| C0                   | 287764      | 1.47      | 26.72       | 15.88      |
| C1                   | 2223151     | 9.30      | 27.32       | 15.88      |
| C2a                  | 4813059     | 18.81     | 26.34       | 15.88      |
| C2b                  | 6255712     | 20.78     | 29.43       | 15.88      |
| C3a                  | 3475239     | 10.98     | 27.39       | 15.88      |
| C3b                  | 3475239     | 10.98     | 27.39       | 15.88      |
| C4a                  | 3115129     | 10.34     | 27.69       | 15.88      |
| C4b                  | 3115129     | 10.34     | 27.69       | 15.88      |
| C5                   | 2098219     | 9.34      | 27.67       | 15.88      |
| user_data_encryption | 1688676     | 5.84      | 27.62       | 15.88      |
| C6                   | 2977263     | 10.37     | 27.45       | 15.88      |
| C7                   | 191104      | 0.88      | 25.92       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.94 KB           | 3154404    | 181968       | 3336372   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972929    | 193312       | 3166241   |
| Π_dec    | 10.69 KB   | 3.75 KB           | 3658524    | 190812       | 3849336   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration  | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | --------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 6689.15 s | 127.00 KB  | 130.81 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 564.46 s  | 10.69 KB   | 11.62 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.20 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.37 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 366.09 s  | 10.69 KB   | 14.44 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 89.40 s   | 10.69 KB   | 14.44 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **46.80 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.80         |
| Committee Setup Completed                                          | `wall_clock` | 18.13        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 564.46       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 6689.15      |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 6689.68      |
| Application CT Gen                                                 | `wall_clock` | 0.29         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 89.40        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 366.09       |
| Entire Test                                                        | `wall_clock` | 7077.00      |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.06    | 10   | 0.58      |
| CalculateDecryptionShare      | 0.17    | 10   | 1.72      |
| CalculateThresholdDecryption  | 0.26    | 1    | 0.26      |
| GenEsiSss                     | 0.39    | 10   | 3.89      |
| GenPkShareAndSkSss            | 0.37    | 10   | 3.65      |
| NodeDkgFold/c2ab_fold         | 30.92   | 10   | 309.16    |
| NodeDkgFold/c3a_fold          | 752.93  | 10   | 7529.29   |
| NodeDkgFold/c3ab_fold         | 12.58   | 10   | 125.78    |
| NodeDkgFold/c3b_fold          | 719.52  | 10   | 7195.20   |
| NodeDkgFold/c4ab_fold         | 13.48   | 10   | 134.81    |
| NodeDkgFold/node_fold         | 24.45   | 10   | 244.54    |
| ZkDecryptedSharesAggregation  | 4.60    | 1    | 4.60      |
| ZkDecryptionAggregation       | 84.47   | 1    | 84.47     |
| ZkDkgAggregation              | 5.54    | 1    | 5.54      |
| ZkDkgShareDecryption          | 94.96   | 20   | 1899.13   |
| ZkNodeDkgFold                 | 1129.40 | 10   | 11294.01  |
| ZkNodesFoldStep               | 6.05    | 8    | 48.43     |
| ZkPkAggregation               | 41.26   | 1    | 41.26     |
| ZkPkBfv                       | 10.52   | 10   | 105.18    |
| ZkPkGeneration                | 79.90   | 10   | 799.03    |
| ZkShareComputation            | 102.51  | 20   | 2050.15   |
| ZkShareEncryption             | 108.42  | 540  | 58548.74  |
| ZkThresholdShareDecryption    | 205.97  | 10   | 2059.68   |
| ZkVerifyShareDecryptionProofs | 0.78    | 10   | 7.82      |
| ZkVerifyShareProofs           | 2.35    | 12   | 28.19     |

Sum of tracked job wall time: **92525.13 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 30.92   | 10   | 309.16    |
| c3a_fold  | 752.93  | 10   | 7529.29   |
| c3ab_fold | 12.58   | 10   | 125.78    |
| c3b_fold  | 719.52  | 10   | 7195.20   |
| c4ab_fold | 13.48   | 10   | 134.81    |
| node_fold | 24.45   | 10   | 244.54    |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 4.60    | 1    | 4.60      |
| ZkDecryptionAggregation      | 84.47   | 1    | 84.47     |
| ZkDkgAggregation             | 5.54    | 1    | 5.54      |
| ZkNodeDkgFold                | 1129.40 | 10   | 11294.01  |
| ZkPkAggregation              | 41.26   | 1    | 41.26     |

Sum of aggregation job tracked time: **11429.88 s** (parallel CPU work; not P1/P2 wall clock).

### Folded on-chain artifacts (exported for Π_DKG / Π_dec gas)

| Artifact              | Proof (bytes) | Public inputs (bytes) |
| --------------------- | ------------- | --------------------- |
| dkg_aggregator        | 10944         | 960                   |
| decryption_aggregator | 10944         | 3840                  |

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
