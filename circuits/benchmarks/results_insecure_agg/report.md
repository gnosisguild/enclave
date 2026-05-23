# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-23 11:19:46 UTC

**Git Branch:** `feat/1549`  
**Git Commit:** `8a841717468169df094b7316e57e1008d4ba34b0`

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
| C0                   | 6847        | 0.12      | 26.22       | 15.88      |
| C1                   | 57818       | 0.34      | 25.94       | 15.88      |
| C2a                  | 41244       | 0.31      | 25.78       | 15.88      |
| C2b                  | 79591       | 0.49      | 26.23       | 15.88      |
| C3a                  | 120114      | 0.57      | 26.46       | 15.88      |
| C3b                  | 120114      | 0.57      | 26.46       | 15.88      |
| C4a                  | 67494       | 0.46      | 26.53       | 15.88      |
| C4b                  | 67494       | 0.46      | 26.53       | 15.88      |
| C5                   | 123624      | 0.55      | 26.63       | 15.88      |
| user_data_encryption | 53732       | 0.33      | 25.67       | 15.88      |
| C6                   | 86927       | 0.53      | 27.16       | 15.88      |
| C7                   | 90841       | 0.46      | 26.43       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3125294    | 176148       | 3301442   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972881    | 170272       | 3143153   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3640985    | 187260       | 3828245   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 144.27 s | 127.00 KB  | 128.19 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 131.81 s | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 0.65 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 0.53 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 53.36 s  | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 49.52 s  | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **20.40 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.66         |
| Committee Setup Completed                                          | `wall_clock` | 20.17        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 131.81       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 144.27       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 144.78       |
| Application CT Gen                                                 | `wall_clock` | 0.01         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 49.52        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 53.36        |
| Entire Test                                                        | `wall_clock` | 220.99       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.01    | 3    | 0.02      |
| CalculateDecryptionShare      | 0.02    | 3    | 0.06      |
| CalculateThresholdDecryption  | 0.02    | 1    | 0.02      |
| GenEsiSss                     | 0.01    | 3    | 0.02      |
| GenPkShareAndSkSss            | 0.01    | 3    | 0.03      |
| NodeDkgFold/c2ab_fold         | 8.17    | 3    | 24.52     |
| NodeDkgFold/c3a_fold          | 35.19   | 3    | 105.58    |
| NodeDkgFold/c3ab_fold         | 7.49    | 3    | 22.47     |
| NodeDkgFold/c3b_fold          | 34.99   | 3    | 104.97    |
| NodeDkgFold/c4ab_fold         | 7.89    | 3    | 23.68     |
| NodeDkgFold/node_fold         | 18.33   | 3    | 55.00     |
| ZkDecryptedSharesAggregation  | 1.56    | 1    | 1.56      |
| ZkDecryptionAggregation       | 47.95   | 1    | 47.95     |
| ZkDkgAggregation              | 19.77   | 1    | 19.77     |
| ZkDkgShareDecryption          | 1.23    | 6    | 7.38      |
| ZkNodeDkgFold                 | 112.07  | 3    | 336.22    |
| ZkPkAggregation               | 0.63    | 1    | 0.63      |
| ZkPkBfv                       | 0.22    | 3    | 0.66      |
| ZkPkGeneration                | 2.36    | 3    | 7.09      |
| ZkShareComputation            | 2.39    | 6    | 14.34     |
| ZkShareEncryption             | 4.00    | 24   | 95.92     |
| ZkThresholdShareDecryption    | 3.39    | 3    | 10.18     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.31      |
| ZkVerifyShareProofs           | 0.27    | 5    | 1.37      |

Sum of tracked job wall time: **879.74 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 8.17    | 3    | 24.52     |
| c3a_fold  | 35.19   | 3    | 105.58    |
| c3ab_fold | 7.49    | 3    | 22.47     |
| c3b_fold  | 34.99   | 3    | 104.97    |
| c4ab_fold | 7.89    | 3    | 23.68     |
| node_fold | 18.33   | 3    | 55.00     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 1.56    | 1    | 1.56      |
| ZkDecryptionAggregation      | 47.95   | 1    | 47.95     |
| ZkDkgAggregation             | 19.77   | 1    | 19.77     |
| ZkNodeDkgFold                | 112.07  | 3    | 336.22    |
| ZkPkAggregation              | 0.63    | 1    | 0.63      |

Sum of aggregation job tracked time: **406.13 s** (parallel CPU work; not P1/P2 wall clock).

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
