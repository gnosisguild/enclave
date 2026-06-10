# Interfold ZK Circuit Benchmarks

**Generated:** 2026-06-09 00:56:42 UTC

**Git Branch:** `main`  
**Git Commit:** `3af9cf30641e51bbecd6a557ae9622113f996104`

**Committee Size:** `H=15`, `N=20`, `T=7`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `secure`                                     |
| BFV preset (artifacts)                                | `secure-8192`                                |
| BFV preset (enum)                                     | `SecureThreshold8192`                        |
| λ (smudging / error)                                  | 50                                           |
| Nodes spawned (builder)                               | 35                                           |
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
| C0                   | 287764      | 1.46      | 28.48       | 15.88      |
| C1                   | 2223151     | 9.20      | 28.00       | 15.88      |
| C2a                  | 10037805    | 37.12     | 26.99       | 15.88      |
| C2b                  | 11480458    | 38.89     | 28.02       | 15.88      |
| C3a                  | 3475239     | 11.02     | 28.39       | 15.88      |
| C3b                  | 3475239     | 11.02     | 28.39       | 15.88      |
| C4a                  | 4724656     | 18.98     | 28.76       | 15.88      |
| C4b                  | 4724656     | 18.98     | 28.76       | 15.88      |
| C5                   | 3682162     | 11.52     | 27.23       | 15.88      |
| user_data_encryption | 1688676     | 5.87      | 28.43       | 15.88      |
| C6                   | 2977263     | 10.41     | 27.41       | 15.88      |
| C7                   | 275103      | 1.34      | 28.08       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 1.59 KB           | 3195070    | 189960       | 3385030   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972845    | 193228       | 3166073   |
| Π_dec    | 10.69 KB   | 4.03 KB           | 3675981    | 194280       | 3870261   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration   | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | ---------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 26958.17 s | 127.00 KB  | 134.00 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 446.24 s   | 10.69 KB   | 12.28 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.16 s    | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.41 s    | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 662.60 s   | 10.69 KB   | 14.72 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 126.71 s   | 10.69 KB   | 14.72 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **62.57 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 5.29         |
| Committee Setup Completed                                          | `wall_clock` | 35.20        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 446.24       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 26958.17     |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 26959.18     |
| Application CT Gen                                                 | `wall_clock` | 0.29         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 126.71       |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 662.60       |
| Entire Test                                                        | `wall_clock` | 27662.57     |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.08    | 20   | 1.52      |
| CalculateDecryptionShare      | 0.19    | 20   | 3.84      |
| CalculateThresholdDecryption  | 0.33    | 1    | 0.33      |
| GenEsiSss                     | 1.23    | 20   | 24.52     |
| GenPkShareAndSkSss            | 2.10    | 20   | 41.97     |
| NodeDkgFold/c2ab_fold         | 28.40   | 20   | 567.96    |
| NodeDkgFold/c3a_fold          | 1715.82 | 20   | 34316.44  |
| NodeDkgFold/c3ab_fold         | 18.00   | 20   | 360.09    |
| NodeDkgFold/c3b_fold          | 1651.78 | 20   | 33035.61  |
| NodeDkgFold/c4ab_fold         | 19.94   | 20   | 398.79    |
| NodeDkgFold/node_fold         | 41.52   | 20   | 830.32    |
| ZkDecryptedSharesAggregation  | 7.28    | 1    | 7.28      |
| ZkDecryptionAggregation       | 118.85  | 1    | 118.85    |
| ZkDkgAggregation              | 5.83    | 1    | 5.83      |
| ZkDkgShareDecryption          | 182.64  | 40   | 7305.56   |
| ZkNodeDkgFold                 | 2893.40 | 20   | 57868.02  |
| ZkNodesFoldStep               | 8.49    | 15   | 127.39    |
| ZkPkAggregation               | 56.74   | 1    | 56.74     |
| ZkPkBfv                       | 11.39   | 20   | 227.82    |
| ZkPkGeneration                | 116.99  | 20   | 2339.75   |
| ZkShareComputation            | 192.23  | 40   | 7689.06   |
| ZkShareEncryption             | 106.84  | 2280 | 243588.85 |
| ZkThresholdShareDecryption    | 284.47  | 20   | 5689.40   |
| ZkVerifyShareDecryptionProofs | 1.37    | 20   | 27.44     |
| ZkVerifyShareProofs           | 5.30    | 22   | 116.58    |

Sum of tracked job wall time: **394749.96 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 28.40   | 20   | 567.96    |
| c3a_fold  | 1715.82 | 20   | 34316.44  |
| c3ab_fold | 18.00   | 20   | 360.09    |
| c3b_fold  | 1651.78 | 20   | 33035.61  |
| c4ab_fold | 19.94   | 20   | 398.79    |
| node_fold | 41.52   | 20   | 830.32    |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 7.28    | 1    | 7.28      |
| ZkDecryptionAggregation      | 118.85  | 1    | 118.85    |
| ZkDkgAggregation             | 5.83    | 1    | 5.83      |
| ZkNodeDkgFold                | 2893.40 | 20   | 57868.02  |
| ZkPkAggregation              | 56.74   | 1    | 56.74     |

Sum of aggregation job tracked time: **58056.72 s** (parallel CPU work; not P1/P2 wall clock).

### Folded on-chain artifacts (exported for Π_DKG / Π_dec gas)

| Artifact              | Proof (bytes) | Public inputs (bytes) |
| --------------------- | ------------- | --------------------- |
| dkg_aggregator        | 10944         | 1632                  |
| decryption_aggregator | 10944         | 4128                  |

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
