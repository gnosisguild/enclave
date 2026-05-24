# Enclave ZK Circuit Benchmarks

**Generated:** 2026-05-23 16:20:50 UTC

**Git Branch:** `feat/1549`  
**Git Commit:** `604ef9af71651ffae19546146b78a7940744741f`

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
| C0                   | 287764      | 1.46      | 26.25       | 15.88      |
| C1                   | 2432074     | 9.41      | 26.79       | 15.88      |
| C2a                  | 1446348     | 5.33      | 26.99       | 15.88      |
| C2b                  | 2889001     | 10.00     | 27.27       | 15.88      |
| C3a                  | 3563512     | 11.07     | 27.07       | 15.88      |
| C3b                  | 3563512     | 11.07     | 27.07       | 15.88      |
| C4a                  | 1961956     | 5.96      | 26.32       | 15.88      |
| C4b                  | 1961956     | 5.96      | 26.32       | 15.88      |
| C5                   | 3719555     | 10.99     | 26.81       | 15.88      |
| user_data_encryption | 1678200     | 5.81      | 26.25       | 15.88      |
| C6                   | 3001847     | 10.43     | 27.26       | 15.88      |
| C7                   | 109424      | 0.51      | 26.42       | 15.88      |

### Artifacts

| Artifact | Proof size | Public input size | Verify gas | Calldata gas | Total gas |
| -------- | ---------- | ----------------- | ---------- | ------------ | --------- |
| Π_DKG    | 10.69 KB   | 0.47 KB           | 3125282    | 176136       | 3301418   |
| Π_user   | 15.88 KB   | 0.12 KB           | 2973001    | 193312       | 3166313   |
| Π_dec    | 10.69 KB   | 3.47 KB           | 3641070    | 187344       | 3828414   |

### Role / Phase / Activity

| Role            | Phase | Activity                                  | Metric         | Duration | Proof size | Bandwidth |
| --------------- | ----- | ----------------------------------------- | -------------- | -------- | ---------- | --------- |
| Each ciphernode | P1    | one-time DKG participation (test harness) | wall_clock     | 616.80 s | 127.00 KB  | 128.56 KB |
| Aggregator      | P2    | C5 + Π_DKG fold (aggregator span)         | wall_clock     | 160.99 s | 10.69 KB   | 11.16 KB  |
| User            | P3    | per user input                            | isolated_nargo | 11.18 s  | 15.88 KB   | 16.00 KB  |
| Each ciphernode | P4    | per computation output (C6)               | isolated_nargo | 10.43 s  | 15.88 KB   | 16.00 KB  |
| Aggregator      | P4    | C7 + Π_dec fold (full publish→aggregate)  | wall_clock     | 186.97 s | 10.69 KB   | 14.16 KB  |
| Aggregator      | P4    | C7 + fold only (pending→plaintext span)   | wall_clock     | 50.52 s  | 10.69 KB   | 14.16 KB  |

_P2 **tracked_job_wall** sum (ZkDkgAggregation + ZkPkAggregation, parallelizable): **43.96 s** — not
comparable to P2 wall_clock row above._

## Integration test (`test_trbfv_actor`)

### End-to-end phase timings (integration test)

| Phase                                                              | Metric       | Duration (s) |
| ------------------------------------------------------------------ | ------------ | ------------ |
| Starting trbfv actor test                                          | `wall_clock` | 0.00         |
| Setup completed                                                    | `wall_clock` | 2.79         |
| Committee Setup Completed                                          | `wall_clock` | 20.37        |
| Committee Finalization Complete                                    | `wall_clock` | 0.00         |
| Aggregator P2: PkAggregation pending -> PublicKeyAggregated (wall) | `wall_clock` | 160.99       |
| ThresholdShares -> PublicKeyAggregated                             | `wall_clock` | 616.80       |
| E3Request -> PublicKeyAggregated                                   | `wall_clock` | 617.32       |
| Application CT Gen                                                 | `wall_clock` | 0.35         |
| Running FHE Application                                            | `wall_clock` | 0.00         |
| Aggregator P4: Aggregation pending -> PlaintextAggregated (wall)   | `wall_clock` | 50.52        |
| Ciphertext published -> PlaintextAggregated                        | `wall_clock` | 186.97       |
| Entire Test                                                        | `wall_clock` | 827.80       |

### Multithread job timings (`tracked_job_wall`)

| Name                          | Avg (s) | Runs | Total (s) |
| ----------------------------- | ------- | ---- | --------- |
| CalculateDecryptionKey        | 0.04    | 3    | 0.12      |
| CalculateDecryptionShare      | 0.16    | 3    | 0.47      |
| CalculateThresholdDecryption  | 0.23    | 1    | 0.23      |
| GenEsiSss                     | 0.08    | 3    | 0.23      |
| GenPkShareAndSkSss            | 0.10    | 3    | 0.31      |
| NodeDkgFold/c2ab_fold         | 7.16    | 3    | 21.48     |
| NodeDkgFold/c3a_fold          | 57.61   | 3    | 172.82    |
| NodeDkgFold/c3ab_fold         | 6.65    | 3    | 19.94     |
| NodeDkgFold/c3b_fold          | 50.04   | 3    | 150.12    |
| NodeDkgFold/c4ab_fold         | 8.45    | 3    | 25.35     |
| NodeDkgFold/node_fold         | 14.89   | 3    | 44.67     |
| ZkDecryptedSharesAggregation  | 2.91    | 1    | 2.91      |
| ZkDecryptionAggregation       | 47.43   | 1    | 47.43     |
| ZkDkgAggregation              | 19.53   | 1    | 19.53     |
| ZkDkgShareDecryption          | 21.92   | 6    | 131.52    |
| ZkNodeDkgFold                 | 144.80  | 3    | 434.40    |
| ZkPkAggregation               | 24.44   | 1    | 24.44     |
| ZkPkBfv                       | 3.56    | 3    | 10.68     |
| ZkPkGeneration                | 55.11   | 3    | 165.34    |
| ZkShareComputation            | 38.70   | 6    | 232.21    |
| ZkShareEncryption             | 121.54  | 36   | 4375.30   |
| ZkThresholdShareDecryption    | 99.20   | 3    | 297.59    |
| ZkVerifyShareDecryptionProofs | 0.13    | 3    | 0.38      |
| ZkVerifyShareProofs           | 0.31    | 5    | 1.56      |

Sum of tracked job wall time: **6179.03 s** — **not** end-to-end latency (jobs run in parallel up to
`BENCHMARK_MULTITHREAD_JOBS`).

### NodeDkgFold sub-steps (`tracked_job_wall`, per fold prove)

| Step      | Avg (s) | Runs | Total (s) |
| --------- | ------- | ---- | --------- |
| c2ab_fold | 7.16    | 3    | 21.48     |
| c3a_fold  | 57.61   | 3    | 172.82    |
| c3ab_fold | 6.65    | 3    | 19.94     |
| c3b_fold  | 50.04   | 3    | 150.12    |
| c4ab_fold | 8.45    | 3    | 25.35     |
| node_fold | 14.89   | 3    | 44.67     |

### Aggregation jobs (`tracked_job_wall`)

| Operation                    | Avg (s) | Runs | Total (s) |
| ---------------------------- | ------- | ---- | --------- |
| ZkDecryptedSharesAggregation | 2.91    | 1    | 2.91      |
| ZkDecryptionAggregation      | 47.43   | 1    | 47.43     |
| ZkDkgAggregation             | 19.53   | 1    | 19.53     |
| ZkNodeDkgFold                | 144.80  | 3    | 434.40    |
| ZkPkAggregation              | 24.44   | 1    | 24.44     |

Sum of aggregation job tracked time: **528.70 s** (parallel CPU work; not P1/P2 wall clock).

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
