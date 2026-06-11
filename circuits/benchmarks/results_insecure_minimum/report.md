# Interfold ZK Circuit Benchmarks

**Generated:** 2026-06-11 09:30:52 UTC

**Git Branch:** `unknown`  
**Git Commit:** `unknown`

**Committee Size:** `H=2`, `N=3`, `T=1`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `insecure`                                   |
| BFV preset (artifacts)                                | `insecure-512`                               |
| BFV preset (enum)                                     | `InsecureThreshold512`                       |
| λ (smudging / error)                                  | 2                                            |
| Nodes spawned (builder)                               | 7                                            |
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

On-chain verify gas: **complete** (CRISP Π_user + Interfold Π_DKG / Π_dec replay).

---

## Measurement methodology

| Metric kind          | Source                                           | Meaning                                                                                    | Do **not** use for                                             |
| -------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| **wall_clock**       | `test_trbfv_actor` phase timers / HLC event span | End-to-end wait in the in-process test harness                                             | Production WAN latency; per-node deployment cost               |
| **isolated_nargo**   | `benchmark_circuit.sh` per circuit               | Single `bb prove` on oracle witness, one circuit at a time                                 | Full protocol pipeline (different witness path)                |
| **tracked_job_wall** | `MultithreadReport` per `ComputeRequest`         | Wall time of each job on the shared Rayon pool (≤ `BENCHMARK_MULTITHREAD_JOBS` concurrent) | End-to-end time — **sums exceed wall clock** when jobs overlap |

**Harness limits (integration):** all ciphernodes share one process and bus
(`network_model: in_process_bus`); sortition registers extra nodes; `testmode_*` enabled; proof
aggregation always enabled. Compare runs only with the same `benchmark_mode`, committee,
`BENCHMARK_MULTITHREAD_JOBS`, commit, and hardware.

---

## Protocol Summary

### Circuit Benchmarks (isolated Nargo + Barretenberg)

Single-circuit `bb prove` on the benchmark oracle witness (not the integration actor pipeline).

| Circuit              | Constraints | Prove (s) | Verify (ms) | Proof (KB) |
| -------------------- | ----------- | --------- | ----------- | ---------- |
| C0                   | 6847        | 0.12      | 25.57       | 15.88      |
| C1                   | 53485       | 0.37      | 27.56       | 15.88      |
| C2a                  | 41244       | 0.31      | 25.02       | 15.88      |
| C2b                  | 79591       | 0.51      | 27.86       | 15.88      |
| C3a                  | 120114      | 0.57      | 26.24       | 15.88      |
| C3b                  | 120114      | 0.57      | 26.24       | 15.88      |
| C4a                  | 62750       | 0.35      | 25.44       | 15.88      |
| C4b                  | 62750       | 0.35      | 25.44       | 15.88      |
| C5                   | 21501       | 0.21      | 25.48       | 15.88      |
| user_data_encryption | 53732       | 0.35      | 26.85       | 15.88      |
| C6                   | 86927       | 0.51      | 25.39       | 15.88      |
| C7                   | 90841       | 0.48      | 25.57       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.38 KB           | 3119493    | 175008       | 3294501   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972941    | 170140       | 3143081   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3641143    | 187416       | 3828559   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 139.74 s | 127.00 KB  | 128.06 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 126.77 s | 10.69 KB   | 11.06 KB  |
| User            | P3    | per user input                            | isolated_nargo | 0.72 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 0.51 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 52.26 s  | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 48.59 s  | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **5.80 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 1.05         |
| Committee Setup Completed                                          | `wall_clock` | 7.03         |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 126.77       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 139.74       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 140.24       |
| Application CT Gen                                                 | `wall_clock` | 0.01         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 48.59        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 52.26        |
| Entire Test                                                        | `wall_clock` | 200.59       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.00    | 3    | 0.01      |
| CalculateDecryptionShare      | 0.02    | 3    | 0.07      |
| CalculateThresholdDecryption  | 0.02    | 1    | 0.02      |
| GenEsiSss                     | 0.01    | 3    | 0.02      |
| GenPkShareAndSkSss            | 0.01    | 3    | 0.03      |
| NodeDkgFold/c2ab_fold         | 18.85   | 3    | 56.55     |
| NodeDkgFold/c3a_fold          | 74.33   | 3    | 223.00    |
| NodeDkgFold/c3ab_fold         | 8.43    | 3    | 25.28     |
| NodeDkgFold/c3b_fold          | 74.51   | 3    | 223.54    |
| NodeDkgFold/c4ab_fold         | 8.16    | 3    | 24.48     |
| NodeDkgFold/node_fold         | 19.42   | 3    | 58.26     |
| ZkDecryptedSharesAggregation  | 1.59    | 1    | 1.59      |
| ZkDecryptionAggregation       | 46.99   | 1    | 46.99     |
| ZkDkgAggregation              | 5.42    | 1    | 5.42      |
| ZkDkgShareDecryption          | 1.04    | 6    | 6.22      |
| ZkNodeDkgFold                 | 110.99  | 3    | 332.97    |
| ZkNodesFoldStep               | 5.07    | 2    | 10.15     |
| ZkPkAggregation               | 0.38    | 1    | 0.38      |
| ZkPkBfv                       | 0.24    | 3    | 0.71      |
| ZkPkGeneration                | 2.35    | 3    | 7.05      |
| ZkShareComputation            | 2.60    | 6    | 15.60     |
| ZkShareEncryption             | 4.28    | 24   | 102.66    |
| ZkThresholdShareDecryption    | 3.38    | 3    | 10.15     |
| ZkVerifyShareDecryptionProofs | 0.15    | 3    | 0.44      |
| ZkVerifyShareProofs           | 0.27    | 5    | 1.34      |

Sum of tracked job wall time: **1152.95 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 18.85   | 3    | 56.55     |
| c3a_fold  | 74.33   | 3    | 223.00    |
| c3ab_fold | 8.43    | 3    | 25.28     |
| c3b_fold  | 74.51   | 3    | 223.54    |
| c4ab_fold | 8.16    | 3    | 24.48     |
| node_fold | 19.42   | 3    | 58.26     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 1.59    | 1    | 1.59      |
| ZkDecryptionAggregation      | 46.99   | 1    | 46.99     |
| ZkDkgAggregation             | 5.42    | 1    | 5.42      |
| ZkNodeDkgFold                | 110.99  | 3    | 332.97    |
| ZkPkAggregation              | 0.38    | 1    | 0.38      |

Sum of aggregation job tracked time: **387.35 s** (parallel CPU work; not P1/P2 wall clock).

### Folded on-chain artifacts (exported for Π_DKG / Π_dec gas)

| Artifact              | Proof (bytes) | Public inputs (bytes) |
| --------------------- | ------------- | --------------------- |
| dkg_aggregator        | 10944         | 384                   |
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
