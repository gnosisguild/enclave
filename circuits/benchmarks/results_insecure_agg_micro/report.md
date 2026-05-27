# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-27 13:55:12 UTC

**Git Branch:** `params/dyn-conf`  
**Git Commit:** `e6189e08754c90981a3c359911e081c5f5ff3a90`

**Committee Size:** `H=3`, `N=3`, `T=1`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `insecure`                                   |
| BFV preset (artifacts)                                | `insecure-512`                               |
| BFV preset (enum)                                     | `InsecureThreshold512`                       |
| λ (smudging / error)                                  | 2                                            |
| Nodes spawned (builder)                               | 20                                           |
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
| C0                   | 6847        | 0.14      | 28.39       | 15.88      |
| C1                   | 57818       | 0.36      | 28.35       | 15.88      |
| C2a                  | 41244       | 0.32      | 27.93       | 15.88      |
| C2b                  | 79591       | 0.53      | 27.27       | 15.88      |
| C3a                  | 120114      | 0.60      | 27.47       | 15.88      |
| C3b                  | 120114      | 0.60      | 27.47       | 15.88      |
| C4a                  | 67494       | 0.46      | 25.09       | 15.88      |
| C4b                  | 67494       | 0.46      | 25.09       | 15.88      |
| C5                   | 123624      | 0.58      | 35.81       | 15.88      |
| user_data_encryption | 53732       | 0.33      | 26.08       | 15.88      |
| C6                   | 86927       | 0.58      | 27.25       | 15.88      |
| C7                   | 90841       | 0.52      | 28.18       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3125379    | 176232       | 3301611   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972989    | 170272       | 3143261   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3640997    | 187272       | 3828269   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 148.93 s | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 136.24 s | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 0.65 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 0.58 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 55.34 s  | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 51.39 s  | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **21.30 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.97         |
| Committee Setup Completed                                          | `wall_clock` | 20.09        |
| Committee Finalization Complete                                    | `wall_clock` | 0.01         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 136.24       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 148.93       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 149.43       |
| Application CT Gen                                                 | `wall_clock` | 0.01         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 51.39        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 55.34        |
| Entire Test                                                        | `wall_clock` | 227.84       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.00    | 3    | 0.01      |
| CalculateDecryptionShare      | 0.02    | 3    | 0.06      |
| CalculateThresholdDecryption  | 0.02    | 1    | 0.02      |
| GenEsiSss                     | 0.01    | 3    | 0.02      |
| GenPkShareAndSkSss            | 0.01    | 3    | 0.03      |
| NodeDkgFold/c2ab_fold         | 8.42    | 3    | 25.26     |
| NodeDkgFold/c3a_fold          | 36.69   | 3    | 110.07    |
| NodeDkgFold/c3ab_fold         | 7.68    | 3    | 23.04     |
| NodeDkgFold/c3b_fold          | 35.82   | 3    | 107.46    |
| NodeDkgFold/c4ab_fold         | 8.05    | 3    | 24.14     |
| NodeDkgFold/node_fold         | 19.28   | 3    | 57.84     |
| ZkDecryptedSharesAggregation  | 1.61    | 1    | 1.61      |
| ZkDecryptionAggregation       | 49.76   | 1    | 49.76     |
| ZkDkgAggregation              | 20.43   | 1    | 20.43     |
| ZkDkgShareDecryption          | 1.32    | 6    | 7.91      |
| ZkNodeDkgFold                 | 115.94  | 3    | 347.81    |
| ZkPkAggregation               | 0.86    | 1    | 0.86      |
| ZkPkBfv                       | 0.23    | 3    | 0.70      |
| ZkPkGeneration                | 2.56    | 3    | 7.69      |
| ZkShareComputation            | 2.55    | 6    | 15.33     |
| ZkShareEncryption             | 4.02    | 24   | 96.43     |
| ZkThresholdShareDecryption    | 3.49    | 3    | 10.48     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.31      |
| ZkVerifyShareProofs           | 0.27    | 5    | 1.35      |

Sum of tracked job wall time: **908.64 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 8.42    | 3    | 25.26     |
| c3a_fold  | 36.69   | 3    | 110.07    |
| c3ab_fold | 7.68    | 3    | 23.04     |
| c3b_fold  | 35.82   | 3    | 107.46    |
| c4ab_fold | 8.05    | 3    | 24.14     |
| node_fold | 19.28   | 3    | 57.84     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 1.61    | 1    | 1.61      |
| ZkDecryptionAggregation      | 49.76   | 1    | 49.76     |
| ZkDkgAggregation             | 20.43   | 1    | 20.43     |
| ZkNodeDkgFold                | 115.94  | 3    | 347.81    |
| ZkPkAggregation              | 0.86    | 1    | 0.86      |

Sum of aggregation job tracked time: **420.48 s** (parallel CPU work; not P1/P2 wall clock).

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
