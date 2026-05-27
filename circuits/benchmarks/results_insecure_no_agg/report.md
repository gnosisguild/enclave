# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-26 22:42:44 UTC

**Git Branch:** `main`  
**Git Commit:** `2e424ac15986037a03a92cb0db1f70d862adc913`

**Committee Size:** `H=8`, `N=10`, `T=4`

## Run configuration

Settings for this benchmark run (integration test + Nargo circuit benches on the same host).

### Integration test (`test_trbfv_actor`)

| Setting                                               | Value                                |
| ----------------------------------------------------- | ------------------------------------ |
| Benchmark mode                                        | `insecure`                           |
| BFV preset (artifacts)                                | `insecure-512`                       |
| BFV preset (enum)                                     | `InsecureThreshold512`               |
| λ (smudging / error)                                  | 2                                    |
| Nodes spawned (builder)                               | 20                                   |
| Network model                                         | `in_process_bus`                     |
| Testmode harness                                      | true                                 |
| `proof_aggregation_enabled`                           | false                                |
| `BENCHMARK_MULTITHREAD_JOBS` (max concurrent ZK jobs) | 13                                   |
| Rayon worker threads                                  | 13                                   |
| CPU cores (host)                                      | 14                                   |
| `dkg_fold_attestation_verifier`                       | _(disabled — proof aggregation off)_ |
| Verbose logging (`run_benchmarks.sh --verbose`)       | false                                |

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
| C0                   | 6847        | 0.12      | 23.35       | 15.88      |
| C1                   | 57818       | 0.33      | 24.25       | 15.88      |
| C2a                  | 130260      | 0.57      | 24.45       | 15.88      |
| C2b                  | 168607      | 0.84      | 24.40       | 15.88      |
| C3a                  | 120114      | 0.62      | 27.47       | 15.88      |
| C3b                  | 120114      | 0.62      | 27.47       | 15.88      |
| C4a                  | 94286       | 0.51      | 25.84       | 15.88      |
| C4b                  | 94286       | 0.51      | 25.84       | 15.88      |
| C5                   | 178906      | 0.85      | 24.73       | 15.88      |
| user_data_encryption | 53732       | 0.35      | 24.86       | 15.88      |
| C6                   | 86927       | 0.52      | 26.69       | 15.88      |
| C7                   | 142855      | 0.74      | 25.41       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 15.88 KB   | 0.28 KB           | N/A        | 181940       | N/A       |
| Π_user   | 15.88 KB   | 0.12 KB           | 2972989    | 170476       | 3143465   |
| Π_dec    | 15.88 KB   | 3.44 KB           | N/A        | 194688       | N/A       |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 22.25 s  | 127.00 KB  | 129.69 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 1.98 s   | 15.88 KB   | 16.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 0.67 s   | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 0.52 s   | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 18.40 s  | 15.88 KB   | 19.31 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 8.25 s   | 15.88 KB   | 19.31 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **1.97 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 3.06         |
| Committee Setup Completed                                          | `wall_clock` | 20.26        |
| Committee Finalization Complete                                    | `wall_clock` | 0.01         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 1.98         |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 22.25        |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 24.87        |
| Application CT Gen                                                 | `wall_clock` | 0.31         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 8.25         |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 18.40        |
| Entire Test                                                        | `wall_clock` | 66.90        |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.12    | 3    | 0.36      |
| CalculateDecryptionShare      | 0.63    | 3    | 1.88      |
| CalculateThresholdDecryption  | 0.56    | 1    | 0.56      |
| GenEsiSss                     | 0.13    | 3    | 0.38      |
| GenPkShareAndSkSss            | 0.26    | 3    | 0.77      |
| ZkDecryptedSharesAggregation  | 8.23    | 1    | 8.23      |
| ZkDkgShareDecryption          | 2.35    | 6    | 14.10     |
| ZkPkAggregation               | 1.97    | 1    | 1.97      |
| ZkPkBfv                       | 0.43    | 3    | 1.29      |
| ZkPkGeneration                | 2.95    | 3    | 8.86      |
| ZkShareComputation            | 3.01    | 6    | 18.04     |
| ZkShareEncryption             | 5.76    | 24   | 138.32    |
| ZkThresholdShareDecryption    | 7.66    | 3    | 22.99     |
| ZkVerifyShareDecryptionProofs | 0.10    | 3    | 0.31      |
| ZkVerifyShareProofs           | 0.26    | 5    | 1.29      |

Sum of tracked job wall time: **219.34 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

_Baseline run: node DKG folds and folded Π_DKG / Π_dec export are disabled. Compare with
`BENCHMARK_PROOF_AGGREGATION=true` (default)._

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 8.23    | 1    | 8.23      |
| ZkPkAggregation              | 1.97    | 1    | 1.97      |

Sum of aggregation job tracked time: **10.20 s** (parallel CPU work; not P1/P2 wall clock).

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
