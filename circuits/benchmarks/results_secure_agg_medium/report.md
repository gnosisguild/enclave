# Enclave ZK Circuit Benchmarks

**Generated:** 2026-06-03 21:00:47 UTC

**Git Branch:** `bench/medium-3mod`  
**Git Commit:** `80a221f2152a8cf4cc4d65d0905c555d18da1f02`

**Committee Size:** `H=8`, `N=10`, `T=4`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `secure`                                     |
| BFV preset (artifacts)                                | `secure-8192`                                |
| BFV preset (enum)                                     | `SecureThreshold8192`                        |
| λ (smudging / error)                                  | 55                                           |
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
| C0                   | 287764      | 1.42      | 24.34       | 15.88      |
| C1                   | 2432076     | 9.28      | 25.01       | 15.88      |
| C2a                  | 4813061     | 17.97     | 24.23       | 15.88      |
| C2b                  | 6255714     | 20.59     | 24.25       | 15.88      |
| C3a                  | 3563517     | 10.72     | 24.68       | 15.88      |
| C3b                  | 3563517     | 10.72     | 24.68       | 15.88      |
| C4a                  | 3115129     | 10.19     | 25.03       | 15.88      |
| C4b                  | 3115129     | 10.19     | 25.03       | 15.88      |
| C5                   | 6073618     | 20.41     | 25.71       | 15.88      |
| user_data_encryption | 1688723     | 5.74      | 25.35       | 15.88      |
| C6                   | 3001845     | 10.24     | 25.30       | 15.88      |
| C7                   | 191104      | 0.86      | 26.53       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.94 KB           | 3154367    | 181920       | 3336287   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972929    | 193348       | 3166277   |
| Π_dec    | 10.69 KB   | 3.75 KB           | 3658402    | 190692       | 3849094   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration  | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | --------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 6558.88 s | 127.00 KB  | 130.81 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 1380.96 s | 10.69 KB   | 11.62 KB  |
| User            | P3    | per user input                            | isolated_nargo | 10.95 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.24 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 382.94 s  | 10.69 KB   | 14.44 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 92.61 s   | 10.69 KB   | 14.44 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **155.61 s** —
not comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.78         |
| Committee Setup Completed                                          | `wall_clock` | 20.09        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 1380.96      |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 6558.88      |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 6559.39      |
| Application CT Gen                                                 | `wall_clock` | 0.28         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 92.61        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 382.94       |
| Entire Test                                                        | `wall_clock` | 6965.49      |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.05    | 10   | 0.46      |
| CalculateDecryptionShare      | 0.16    | 10   | 1.58      |
| CalculateThresholdDecryption  | 0.25    | 1    | 0.25      |
| GenEsiSss                     | 0.36    | 10   | 3.65      |
| GenPkShareAndSkSss            | 0.26    | 10   | 2.60      |
| NodeDkgFold/c2ab_fold         | 25.92   | 10   | 259.25    |
| NodeDkgFold/c3a_fold          | 674.19  | 10   | 6741.94   |
| NodeDkgFold/c3ab_fold         | 20.81   | 10   | 208.08    |
| NodeDkgFold/c3b_fold          | 639.78  | 10   | 6397.83   |
| NodeDkgFold/c4ab_fold         | 20.69   | 10   | 206.88    |
| NodeDkgFold/node_fold         | 39.74   | 10   | 397.38    |
| ZkDecryptedSharesAggregation  | 4.73    | 1    | 4.73      |
| ZkDecryptionAggregation       | 87.50   | 1    | 87.50     |
| ZkDkgAggregation              | 41.30   | 1    | 41.30     |
| ZkDkgShareDecryption          | 90.86   | 20   | 1817.28   |
| ZkNodeDkgFold                 | 1421.14 | 10   | 14211.40  |
| ZkPkAggregation               | 114.31  | 1    | 114.31    |
| ZkPkBfv                       | 10.21   | 10   | 102.12    |
| ZkPkGeneration                | 121.56  | 10   | 1215.57   |
| ZkShareComputation            | 96.21   | 20   | 1924.15   |
| ZkShareEncryption             | 106.26  | 540  | 57380.53  |
| ZkThresholdShareDecryption    | 192.97  | 10   | 1929.69   |
| ZkVerifyShareDecryptionProofs | 0.51    | 10   | 5.13      |
| ZkVerifyShareProofs           | 2.06    | 12   | 24.67     |

Sum of tracked job wall time: **93078.28 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 25.92   | 10   | 259.25    |
| c3a_fold  | 674.19  | 10   | 6741.94   |
| c3ab_fold | 20.81   | 10   | 208.08    |
| c3b_fold  | 639.78  | 10   | 6397.83   |
| c4ab_fold | 20.69   | 10   | 206.88    |
| node_fold | 39.74   | 10   | 397.38    |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 4.73    | 1    | 4.73      |
| ZkDecryptionAggregation      | 87.50   | 1    | 87.50     |
| ZkDkgAggregation             | 41.30   | 1    | 41.30     |
| ZkNodeDkgFold                | 1421.14 | 10   | 14211.40  |
| ZkPkAggregation              | 114.31  | 1    | 114.31    |

Sum of aggregation job tracked time: **14459.24 s** (parallel CPU work; not P1/P2 wall clock).

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
