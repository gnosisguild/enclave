# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-27 15:42:56 UTC

**Git Branch:** `params/dyn-conf`  
**Git Commit:** `b015a7ed6e5c7c989f2fd267cf14647865854100`

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
| C0                   | 287764      | 1.71      | 28.61       | 15.88      |
| C1                   | 2432074     | 9.68      | 25.89       | 15.88      |
| C2a                  | 1446348     | 5.54      | 32.30       | 15.88      |
| C2b                  | 2889001     | 10.24     | 26.92       | 15.88      |
| C3a                  | 3563512     | 11.18     | 24.78       | 15.88      |
| C3b                  | 3563512     | 11.18     | 24.78       | 15.88      |
| C4a                  | 1961956     | 5.92      | 25.28       | 15.88      |
| C4b                  | 1961956     | 5.92      | 25.28       | 15.88      |
| C5                   | 3719555     | 11.25     | 26.59       | 15.88      |
| user_data_encryption | 1678200     | 5.92      | 26.34       | 15.88      |
| C6                   | 3001847     | 11.55     | 27.82       | 15.88      |
| C7                   | 109424      | 0.53      | 29.11       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3125318    | 176172       | 3301490   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972965    | 193216       | 3166181   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3640997    | 187272       | 3828269   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 646.10 s | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 166.66 s | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.52 s  | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 11.55 s  | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 152.64 s | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 51.08 s  | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **45.62 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 3.00         |
| Committee Setup Completed                                          | `wall_clock` | 20.10        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 166.66       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 646.10       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 646.66       |
| Application CT Gen                                                 | `wall_clock` | 0.32         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 51.08        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 152.64       |
| Entire Test                                                        | `wall_clock` | 822.72       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.04    | 3    | 0.12      |
| CalculateDecryptionShare      | 0.16    | 3    | 0.47      |
| CalculateThresholdDecryption  | 0.24    | 1    | 0.24      |
| GenEsiSss                     | 0.06    | 3    | 0.18      |
| GenPkShareAndSkSss            | 0.09    | 3    | 0.28      |
| NodeDkgFold/c2ab_fold         | 7.48    | 3    | 22.44     |
| NodeDkgFold/c3a_fold          | 59.44   | 3    | 178.33    |
| NodeDkgFold/c3ab_fold         | 6.88    | 3    | 20.64     |
| NodeDkgFold/c3b_fold          | 52.51   | 3    | 157.52    |
| NodeDkgFold/c4ab_fold         | 8.78    | 3    | 26.34     |
| NodeDkgFold/node_fold         | 15.67   | 3    | 47.02     |
| ZkDecryptedSharesAggregation  | 2.76    | 1    | 2.76      |
| ZkDecryptionAggregation       | 48.18   | 1    | 48.18     |
| ZkDkgAggregation              | 20.01   | 1    | 20.01     |
| ZkDkgShareDecryption          | 28.81   | 6    | 172.87    |
| ZkNodeDkgFold                 | 150.77  | 3    | 452.30    |
| ZkPkAggregation               | 25.61   | 1    | 25.61     |
| ZkPkBfv                       | 3.61    | 3    | 10.82     |
| ZkPkGeneration                | 108.80  | 3    | 326.40    |
| ZkShareComputation            | 75.60   | 6    | 453.58    |
| ZkShareEncryption             | 124.99  | 36   | 4499.53   |
| ZkThresholdShareDecryption    | 98.95   | 3    | 296.84    |
| ZkVerifyShareDecryptionProofs | 0.12    | 3    | 0.36      |
| ZkVerifyShareProofs           | 0.35    | 5    | 1.77      |

Sum of tracked job wall time: **6764.60 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 7.48    | 3    | 22.44     |
| c3a_fold  | 59.44   | 3    | 178.33    |
| c3ab_fold | 6.88    | 3    | 20.64     |
| c3b_fold  | 52.51   | 3    | 157.52    |
| c4ab_fold | 8.78    | 3    | 26.34     |
| node_fold | 15.67   | 3    | 47.02     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 2.76    | 1    | 2.76      |
| ZkDecryptionAggregation      | 48.18   | 1    | 48.18     |
| ZkDkgAggregation             | 20.01   | 1    | 20.01     |
| ZkNodeDkgFold                | 150.77  | 3    | 452.30    |
| ZkPkAggregation              | 25.61   | 1    | 25.61     |

Sum of aggregation job tracked time: **548.86 s** (parallel CPU work; not P1/P2 wall clock).

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
