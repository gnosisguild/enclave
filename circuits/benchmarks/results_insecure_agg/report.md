# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-27 08:29:50 UTC

**Git Branch:** `unknown`  
**Git Commit:** `unknown`

**Committee Size:** `H=8`, `N=10`, `T=4`

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
| C0                   | 6847        | 0.13      | 27.44       | 15.88      |
| C1                   | 57818       | 0.34      | 24.45       | 15.88      |
| C2a                  | 130260      | 0.57      | 24.70       | 15.88      |
| C2b                  | 168607      | 0.83      | 23.75       | 15.88      |
| C3a                  | 120114      | 0.54      | 24.92       | 15.88      |
| C3b                  | 120114      | 0.54      | 24.92       | 15.88      |
| C4a                  | 94286       | 0.49      | 25.37       | 15.88      |
| C4b                  | 94286       | 0.49      | 25.37       | 15.88      |
| C5                   | 178906      | 0.84      | 24.56       | 15.88      |
| user_data_encryption | 53732       | 0.33      | 24.83       | 15.88      |
| C6                   | 86927       | 0.51      | 24.20       | 15.88      |
| C7                   | 142855      | 0.74      | 24.47       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.94 KB           | N/A        | 181908       | N/A       |
| Π_user   | 15.88 KB   | 0.12 KB           | 2973025    | 170356       | 3143381   |
| Π_dec    | 10.69 KB   | 3.75 KB           | N/A        | 190836       | N/A       |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration  | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | --------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 1206.55 s | 127.00 KB  | 129.69 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 1052.53 s | 10.69 KB   | 11.62 KB  |
| User            | P3    | per user input                            | isolated_nargo | 0.64 s    | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 0.51 s    | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 96.33 s   | 10.69 KB   | 14.44 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 85.72 s   | 10.69 KB   | 14.44 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **46.81 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.98         |
| Committee Setup Completed                                          | `wall_clock` | 20.09        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 1052.53      |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 1206.55      |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 1207.06      |
| Application CT Gen                                                 | `wall_clock` | 0.01         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 85.72        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 96.33        |
| Entire Test                                                        | `wall_clock` | 1326.48      |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.36    | 10   | 3.64      |
| CalculateDecryptionShare      | 0.02    | 10   | 0.22      |
| CalculateThresholdDecryption  | 0.02    | 1    | 0.02      |
| GenEsiSss                     | 0.04    | 10   | 0.39      |
| GenPkShareAndSkSss            | 0.02    | 10   | 0.23      |
| NodeDkgFold/c2ab_fold         | 24.51   | 10   | 245.13    |
| NodeDkgFold/c3a_fold          | 439.97  | 10   | 4399.66   |
| NodeDkgFold/c3ab_fold         | 22.83   | 10   | 228.26    |
| NodeDkgFold/c3b_fold          | 445.88  | 10   | 4458.84   |
| NodeDkgFold/c4ab_fold         | 22.19   | 10   | 221.95    |
| NodeDkgFold/node_fold         | 59.42   | 10   | 594.23    |
| ZkDecryptedSharesAggregation  | 2.17    | 1    | 2.17      |
| ZkDecryptionAggregation       | 83.53   | 1    | 83.53     |
| ZkDkgAggregation              | 39.89   | 1    | 39.89     |
| ZkDkgShareDecryption          | 3.11    | 20   | 62.15     |
| ZkNodeDkgFold                 | 1014.81 | 10   | 10148.10  |
| ZkPkAggregation               | 6.92    | 1    | 6.92      |
| ZkPkBfv                       | 0.53    | 10   | 5.33      |
| ZkPkGeneration                | 2.52    | 10   | 25.20     |
| ZkShareComputation            | 5.11    | 20   | 102.19    |
| ZkShareEncryption             | 4.41    | 360  | 1588.53   |
| ZkThresholdShareDecryption    | 7.85    | 10   | 78.45     |
| ZkVerifyShareDecryptionProofs | 1.22    | 10   | 12.19     |
| ZkVerifyShareProofs           | 1.73    | 12   | 20.75     |

Sum of tracked job wall time: **22327.96 s** — **not** end-to-end latency (jobs run in parallel up
to `BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 24.51   | 10   | 245.13    |
| c3a_fold  | 439.97  | 10   | 4399.66   |
| c3ab_fold | 22.83   | 10   | 228.26    |
| c3b_fold  | 445.88  | 10   | 4458.84   |
| c4ab_fold | 22.19   | 10   | 221.95    |
| node_fold | 59.42   | 10   | 594.23    |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 2.17    | 1    | 2.17      |
| ZkDecryptionAggregation      | 83.53   | 1    | 83.53     |
| ZkDkgAggregation             | 39.89   | 1    | 39.89     |
| ZkNodeDkgFold                | 1014.81 | 10   | 10148.10  |
| ZkPkAggregation              | 6.92    | 1    | 6.92      |

Sum of aggregation job tracked time: **10280.60 s** (parallel CPU work; not P1/P2 wall clock).

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
