# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-22 17:39:46 UTC

**Git Branch:** `feat/1549`  
**Git Commit:** `f5c2fef8490fc34fe7357743220321af9626c879`

**Committee Size:** `H=3`, `N=3`, `T=1`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                |
| ----------------------------------------------------- | ------------------------------------ |
| Benchmark mode                                        | `secure`                             |
| BFV preset (artifacts)                                | `secure-8192`                        |
| BFV preset (enum)                                     | `SecureThreshold8192`                |
| λ (smudging / error)                                  | 60                                   |
| Nodes spawned (builder)                               | 20                                   |
| Network model                                         | `in_process_bus`                     |
| Testmode harness                                      | true                                 |
| `proof_aggregation_enabled`                           | false                                |
| `BENCHMARK_MULTITHREAD_JOBS` (max concurrent ZK jobs) | 13                                   |
| Rayon worker threads                                  | 13                                   |
| CPU cores (host)                                      | 14                                   |
| `dkg_fold_attestation_verifier`                       | _(disabled — proof aggregation off)_ |
| Verbose logging (`run_benchmarks.sh --verbose`)       | true                                 |

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

> **Incomplete on-chain verify gas:** 2 of 3 artifact verify-gas values are **N/A**. Re-run
> `./run_benchmarks.sh` and ensure `extract_crisp_verify_gas.sh` completes (CRISP test +
> `test_trbfv_actor` + EVM replay). Calldata gas alone is not sufficient for audit sign-off.

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
| C0                   | 287764      | 1.51      | 28.59       | 15.88      |
| C1                   | 2432074     | 10.13     | 43.32       | 15.88      |
| C2a                  | 3879330     | 11.31     | 27.05       | 15.88      |
| C2b                  | 5739750     | 20.07     | 29.21       | 15.88      |
| C3a                  | 3764144     | 12.08     | 27.24       | 15.88      |
| C3b                  | 3764144     | 12.08     | 27.24       | 15.88      |
| C4a                  | 2564001     | 9.76      | 28.00       | 15.88      |
| C4b                  | 2564001     | 9.76      | 28.00       | 15.88      |
| C5                   | 4395328     | 18.87     | 27.59       | 15.88      |
| user_data_encryption | 1678200     | 6.18      | 28.38       | 15.88      |
| C6                   | 3001847     | 10.78     | 27.88       | 15.88      |
| C7                   | 128310      | 0.55      | 27.37       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 15.88 KB   | 0.12 KB           | N/A        | 202300       | N/A       |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972965    | 193324       | 3166289   |
| Π_dec    | 15.88 KB   | 3.25 KB           | N/A        | 188244       | N/A       |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration  | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | --------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 1226.64 s | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 49.16 s   | 15.88 KB   | 16.00 KB  |
| User            | P3    | per user input                            | isolated_nargo | 12.00 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.78 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 331.86 s  | 15.88 KB   | 19.12 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 19.35 s   | 15.88 KB   | 19.12 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **49.00 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 3.29         |
| Committee Setup Completed                                          | `wall_clock` | 20.25        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 49.16        |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 1226.64      |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 1233.99      |
| Application CT Gen                                                 | `wall_clock` | 7.69         |
| Running FHE Application                                            | `wall_clock` | 0.08         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 19.35        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 331.86       |
| Entire Test                                                        | `wall_clock` | 1597.16      |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.62    | 3    | 1.87      |
| CalculateDecryptionShare      | 2.17    | 3    | 6.52      |
| CalculateThresholdDecryption  | 1.95    | 1    | 1.95      |
| GenEsiSss                     | 0.75    | 3    | 2.25      |
| GenPkShareAndSkSss            | 1.55    | 3    | 4.65      |
| ZkDecryptedSharesAggregation  | 18.93   | 1    | 18.93     |
| ZkDkgShareDecryption          | 55.89   | 6    | 335.31    |
| ZkPkAggregation               | 49.00   | 1    | 49.00     |
| ZkPkBfv                       | 6.07    | 3    | 18.22     |
| ZkPkGeneration                | 380.69  | 3    | 1142.08   |
| ZkShareComputation            | 118.56  | 6    | 711.37    |
| ZkShareEncryption             | 297.63  | 36   | 10714.67  |
| ZkThresholdShareDecryption    | 299.47  | 3    | 898.41    |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.29      |
| ZkVerifyShareProofs           | 0.30    | 5    | 1.48      |

Sum of tracked job wall time: **13907.01 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

_Baseline run: node DKG folds and folded Π_DKG / Π_dec export are disabled. Compare with
`BENCHMARK_PROOF_AGGREGATION=true` (default)._

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 18.93   | 1    | 18.93     |
| ZkPkAggregation              | 49.00   | 1    | 49.00     |

Sum of aggregation job tracked time: **67.94 s** (parallel CPU work; not P1/P2 wall clock).

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
