# Enclave ZK Circuit Benchmarks

**Generated:** 2026-06-03 11:53:20 UTC

**Git Branch:** `main`  
**Git Commit:** `a9ed545dc11fd260d1f1f1516c8d7caffe2f1e02`

**Committee Size:** `H=8`, `N=10`, `T=4`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                        |
| ----------------------------------------------------- | -------------------------------------------- |
| Benchmark mode                                        | `secure`                                     |
| BFV preset (artifacts)                                | `secure-8192`                                |
| BFV preset (enum)                                     | `SecureThreshold8192`                        |
| λ (smudging / error)                                  | 40                                           |
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
| C0                   | 287764      | 1.49      | 23.95       | 15.88      |
| C1                   | 1394755     | 5.42      | 26.12       | 15.88      |
| C2a                  | 3212793     | 10.98     | 28.18       | 15.88      |
| C2b                  | 3972121     | 11.71     | 26.37       | 15.88      |
| C3a                  | 3563521     | 11.60     | 26.37       | 15.88      |
| C3b                  | 3563521     | 11.60     | 26.37       | 15.88      |
| C4a                  | 2077731     | 6.29      | 26.02       | 15.88      |
| C4b                  | 2077731     | 6.29      | 26.02       | 15.88      |
| C5                   | 4050492     | 12.12     | 29.21       | 15.88      |
| user_data_encryption | 1169783     | 5.22      | 26.55       | 15.88      |
| C6                   | 2002335     | 6.36      | 25.29       | 15.88      |
| C7                   | 146290      | 0.80      | 27.17       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.94 KB           | 3154355    | 181908       | 3336263   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2973013    | 193324       | 3166337   |
| Π_dec    | 10.69 KB   | 3.75 KB           | 3658609    | 190896       | 3849505   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration  | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | --------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 4482.84 s | 127.00 KB  | 129.69 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 1001.54 s | 10.69 KB   | 11.62 KB  |
| User            | P3    | per user input                            | isolated_nargo | 8.52 s    | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 6.36 s    | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 269.96 s  | 10.69 KB   | 14.44 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 87.48 s   | 10.69 KB   | 14.44 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **117.12 s** —
not comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 3.00         |
| Committee Setup Completed                                          | `wall_clock` | 20.10        |
| Committee Finalization Complete                                    | `wall_clock` | 0.01         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 1001.54      |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 4482.84      |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 4483.35      |
| Application CT Gen                                                 | `wall_clock` | 0.21         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 87.48        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 269.96       |
| Entire Test                                                        | `wall_clock` | 4776.63      |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.03    | 10   | 0.31      |
| CalculateDecryptionShare      | 0.07    | 10   | 0.73      |
| CalculateThresholdDecryption  | 0.14    | 1    | 0.14      |
| GenEsiSss                     | 0.24    | 10   | 2.37      |
| GenPkShareAndSkSss            | 0.39    | 10   | 3.88      |
| NodeDkgFold/c2ab_fold         | 26.90   | 10   | 269.01    |
| NodeDkgFold/c3a_fold          | 444.97  | 10   | 4449.67   |
| NodeDkgFold/c3ab_fold         | 21.59   | 10   | 215.89    |
| NodeDkgFold/c3b_fold          | 435.12  | 10   | 4351.16   |
| NodeDkgFold/c4ab_fold         | 22.91   | 10   | 229.09    |
| NodeDkgFold/node_fold         | 41.98   | 10   | 419.76    |
| ZkDecryptedSharesAggregation  | 3.23    | 1    | 3.23      |
| ZkDecryptionAggregation       | 84.01   | 1    | 84.01     |
| ZkDkgAggregation              | 40.24   | 1    | 40.24     |
| ZkDkgShareDecryption          | 51.77   | 20   | 1035.34   |
| ZkNodeDkgFold                 | 993.46  | 10   | 9934.64   |
| ZkPkAggregation               | 76.88   | 1    | 76.88     |
| ZkPkBfv                       | 10.51   | 10   | 105.14    |
| ZkPkGeneration                | 58.39   | 10   | 583.86    |
| ZkShareComputation            | 53.98   | 20   | 1079.67   |
| ZkShareEncryption             | 110.17  | 360  | 39659.72  |
| ZkThresholdShareDecryption    | 136.37  | 10   | 1363.72   |
| ZkVerifyShareDecryptionProofs | 0.81    | 10   | 8.05      |
| ZkVerifyShareProofs           | 1.83    | 12   | 21.91     |

Sum of tracked job wall time: **63938.44 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 26.90   | 10   | 269.01    |
| c3a_fold  | 444.97  | 10   | 4449.67   |
| c3ab_fold | 21.59   | 10   | 215.89    |
| c3b_fold  | 435.12  | 10   | 4351.16   |
| c4ab_fold | 22.91   | 10   | 229.09    |
| node_fold | 41.98   | 10   | 419.76    |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 3.23    | 1    | 3.23      |
| ZkDecryptionAggregation      | 84.01   | 1    | 84.01     |
| ZkDkgAggregation             | 40.24   | 1    | 40.24     |
| ZkNodeDkgFold                | 993.46  | 10   | 9934.64   |
| ZkPkAggregation              | 76.88   | 1    | 76.88     |

Sum of aggregation job tracked time: **10139.00 s** (parallel CPU work; not P1/P2 wall clock).

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
