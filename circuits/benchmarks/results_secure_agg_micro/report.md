# Enclave ZK Circuit Benchmarks

**Generated:** 2026-06-03 14:55:43 UTC

**Git Branch:** `bench/medium-3mod`  
**Git Commit:** `80a221f2152a8cf4cc4d65d0905c555d18da1f02`

**Committee Size:** `H=3`, `N=3`, `T=1`

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
| C0                   | 287764      | 1.47      | 25.50       | 15.88      |
| C1                   | 2432076     | 9.59      | 26.57       | 15.88      |
| C2a                  | 1446350     | 5.29      | 24.59       | 15.88      |
| C2b                  | 2889003     | 9.89      | 25.78       | 15.88      |
| C3a                  | 3563517     | 11.06     | 25.04       | 15.88      |
| C3b                  | 3563517     | 11.06     | 25.04       | 15.88      |
| C4a                  | 1961956     | 6.23      | 26.22       | 15.88      |
| C4b                  | 1961956     | 6.23      | 26.22       | 15.88      |
| C5                   | 3719555     | 11.57     | 27.49       | 15.88      |
| user_data_encryption | 1688723     | 5.84      | 25.83       | 15.88      |
| C6                   | 3001845     | 10.73     | 27.34       | 15.88      |
| C7                   | 109424      | 0.52      | 26.11       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3125318    | 176172       | 3301490   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2973061    | 193372       | 3166433   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3641070    | 187344       | 3828414   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 409.87 s | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 132.44 s | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.42 s  | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.73 s  | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 115.27 s | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 49.88 s  | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **37.46 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.93         |
| Committee Setup Completed                                          | `wall_clock` | 20.10        |
| Committee Finalization Complete                                    | `wall_clock` | 0.01         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 132.44       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 409.87       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 410.39       |
| Application CT Gen                                                 | `wall_clock` | 0.22         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 49.88        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 115.27       |
| Entire Test                                                        | `wall_clock` | 548.91       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.02    | 3    | 0.05      |
| CalculateDecryptionShare      | 0.07    | 3    | 0.21      |
| CalculateThresholdDecryption  | 0.13    | 1    | 0.13      |
| GenEsiSss                     | 0.04    | 3    | 0.13      |
| GenPkShareAndSkSss            | 0.06    | 3    | 0.19      |
| NodeDkgFold/c2ab_fold         | 8.98    | 3    | 26.95     |
| NodeDkgFold/c3a_fold          | 39.37   | 3    | 118.12    |
| NodeDkgFold/c3ab_fold         | 6.94    | 3    | 20.83     |
| NodeDkgFold/c3b_fold          | 36.05   | 3    | 108.16    |
| NodeDkgFold/c4ab_fold         | 8.61    | 3    | 25.82     |
| NodeDkgFold/node_fold         | 15.40   | 3    | 46.21     |
| ZkDecryptedSharesAggregation  | 2.07    | 1    | 2.07      |
| ZkDecryptionAggregation       | 47.71   | 1    | 47.71     |
| ZkDkgAggregation              | 19.71   | 1    | 19.71     |
| ZkDkgShareDecryption          | 16.06   | 6    | 96.35     |
| ZkNodeDkgFold                 | 115.36  | 3    | 346.09    |
| ZkPkAggregation               | 17.75   | 1    | 17.75     |
| ZkPkBfv                       | 3.48    | 3    | 10.44     |
| ZkPkGeneration                | 52.17   | 3    | 156.51    |
| ZkShareComputation            | 15.19   | 6    | 91.13     |
| ZkShareEncryption             | 96.39   | 24   | 2313.27   |
| ZkThresholdShareDecryption    | 63.50   | 3    | 190.49    |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.30      |
| ZkVerifyShareProofs           | 0.22    | 5    | 1.11      |

Sum of tracked job wall time: **3639.73 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 8.98    | 3    | 26.95     |
| c3a_fold  | 39.37   | 3    | 118.12    |
| c3ab_fold | 6.94    | 3    | 20.83     |
| c3b_fold  | 36.05   | 3    | 108.16    |
| c4ab_fold | 8.61    | 3    | 25.82     |
| node_fold | 15.40   | 3    | 46.21     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 2.07    | 1    | 2.07      |
| ZkDecryptionAggregation      | 47.71   | 1    | 47.71     |
| ZkDkgAggregation             | 19.71   | 1    | 19.71     |
| ZkNodeDkgFold                | 115.36  | 3    | 346.09    |
| ZkPkAggregation              | 17.75   | 1    | 17.75     |

Sum of aggregation job tracked time: **433.32 s** (parallel CPU work; not P1/P2 wall clock).

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
