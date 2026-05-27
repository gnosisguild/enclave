# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-27 15:16:58 UTC

**Git Branch:** `params/dyn-conf`  
**Git Commit:** `b015a7ed6e5c7c989f2fd267cf14647865854100`

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
| C0                   | 287764      | 1.53      | 27.12       | 15.88      |
| C1                   | 2432074     | 9.73      | 28.71       | 15.88      |
| C2a                  | 1446348     | 5.60      | 26.71       | 15.88      |
| C2b                  | 2889001     | 10.88     | 26.18       | 15.88      |
| C3a                  | 3563512     | 11.08     | 25.14       | 15.88      |
| C3b                  | 3563512     | 11.08     | 25.14       | 15.88      |
| C4a                  | 1961956     | 6.03      | 26.05       | 15.88      |
| C4b                  | 1961956     | 6.03      | 26.05       | 15.88      |
| C5                   | 3719555     | 11.47     | 28.81       | 15.88      |
| user_data_encryption | 1678200     | 5.78      | 26.54       | 15.88      |
| C6                   | 3001847     | 10.38     | 26.76       | 15.88      |
| C7                   | 109424      | 0.52      | 26.43       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 15.88 KB   | 0.12 KB           | N/A        | 198016       | N/A       |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972869    | 193264       | 3166133   |
| Π_dec    | 15.88 KB   | 3.25 KB           | N/A        | 188220       | N/A       |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 466.55 s | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 12.06 s  | 15.88 KB   | 16.00 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.29 s  | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.38 s  | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 100.62 s | 15.88 KB   | 19.12 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 2.93 s   | 15.88 KB   | 19.12 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **12.01 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 3.03         |
| Committee Setup Completed                                          | `wall_clock` | 20.11        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 12.06        |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 466.55       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 467.10       |
| Application CT Gen                                                 | `wall_clock` | 0.32         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 2.93         |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 100.62       |
| Entire Test                                                        | `wall_clock` | 591.19       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.05    | 3    | 0.16      |
| CalculateDecryptionShare      | 0.16    | 3    | 0.47      |
| CalculateThresholdDecryption  | 0.24    | 1    | 0.24      |
| GenEsiSss                     | 0.07    | 3    | 0.21      |
| GenPkShareAndSkSss            | 0.10    | 3    | 0.29      |
| ZkDecryptedSharesAggregation  | 2.80    | 1    | 2.80      |
| ZkDkgShareDecryption          | 21.82   | 6    | 130.93    |
| ZkPkAggregation               | 12.01   | 1    | 12.01     |
| ZkPkBfv                       | 3.53    | 3    | 10.58     |
| ZkPkGeneration                | 82.94   | 3    | 248.82    |
| ZkShareComputation            | 70.24   | 6    | 421.41    |
| ZkShareEncryption             | 120.77  | 36   | 4347.71   |
| ZkThresholdShareDecryption    | 95.06   | 3    | 285.19    |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.30      |
| ZkVerifyShareProofs           | 0.32    | 5    | 1.61      |

Sum of tracked job wall time: **5462.73 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

_Baseline run: node DKG folds and folded Π_DKG / Π_dec export are disabled. Compare with
`BENCHMARK_PROOF_AGGREGATION=true` (default)._

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 2.80    | 1    | 2.80      |
| ZkPkAggregation              | 12.01   | 1    | 12.01     |

Sum of aggregation job tracked time: **14.81 s** (parallel CPU work; not P1/P2 wall clock).

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
