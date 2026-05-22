# Benchmarks

Scripts to compile and time Nargo packages listed in `config.json` (`results_*/report.md`).

|                       |                                                     |
| --------------------- | --------------------------------------------------- |
| **Circuits overview** | [README](../README.md)                              |
| **Docs**              | [Noir Circuits](../../docs/pages/noir-circuits.mdx) |

## Run

From this directory:

```bash
./run_benchmarks.sh
./run_benchmarks.sh --mode secure --circuit dkg/pk
./run_benchmarks.sh --skip-compile
```

Options and secure-only **config** circuit behavior are documented in the script and `config.json`.

### Proof aggregation and folding (integration)

The gas / integration stage runs `cargo test -p e3-tests test_trbfv_actor` with **proof aggregation
enabled by default** (`E3Requested.proof_aggregation_enabled = true`): per-node `ZkNodeDkgFold`,
fold attestations (EIP-712 against `DkgFoldAttestationVerifier`), and exported folded
`dkg_aggregator` / `decryption_aggregator` proofs for Π_DKG / Π_dec on-chain gas.

| Flag / env                                              | Effect                                                                               |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `--proof-aggregation on` (default)                      | Full fold + aggregator path; folded artifacts in report                              |
| `--proof-aggregation off` / `--no-proof-aggregation`    | Baseline without node folds / folded export                                          |
| `BENCHMARK_PROOF_AGGREGATION`                           | Same as above when calling `extract_crisp_verify_gas.sh` directly                    |
| `BENCHMARK_MULTITHREAD_JOBS=N` / `--multithread-jobs N` | Rayon concurrent ZK jobs (default `1`)                                               |
| `BENCHMARK_DKG_FOLD_ATTESTATION_VERIFIER=0x…`           | EIP-712 verifying contract for fold attestations (default: localhost deploy address) |

**Default output directories** (under `circuits/benchmarks/`; aggregation on/off no longer
overwrites the same folder):

| Mode       | Proof aggregation on (default) | Proof aggregation off (`--no-proof-aggregation`) |
| ---------- | ------------------------------ | ------------------------------------------------ |
| `insecure` | `results_insecure_agg/`        | `results_insecure_no_agg/`                       |
| `secure`   | `results_secure_agg/`          | `results_secure_no_agg/`                         |

**A/B comparison** (from `circuits/benchmarks`):

```bash
./run_benchmarks.sh --mode insecure --no-proof-aggregation
./run_benchmarks.sh --mode insecure
# Compare results_insecure_no_agg/report.md vs results_insecure_agg/report.md
```

`report.md` includes **Audit status**, **Measurement methodology** (metric kinds), labeled **Role /
Phase** rows (`wall_clock` vs `isolated_nargo` vs `tracked_job_wall`), **NodeDkgFold sub-steps**,
and **Folded on-chain artifacts** when `integration_summary` is present. Verify gas must be complete
(not N/A) for audit sign-off.

### What gets stored (secure / insecure)

A full `./run_benchmarks.sh --mode <mode>` run writes to `results_<mode>_agg/` or
`results_<mode>_no_agg/` (see table above):

- `raw/*.json` — Nargo timing + artifact sizes (source for the **Circuit Benchmarks** table)
- `crisp_verify_gas.json` — verify gas, calldata gas, artifact sizes, and **`integration_summary`**
  from `test_trbfv_actor` when gas extraction succeeds
- `integration_summary.json` — snapshot of `integration_summary` (phase timings, folded proofs,
  multithread / operation timings)
- `benchmark_run_meta.json` — CLI flags (mode, proof aggregation, multithread jobs, verbose)
- `report.md` — rendered summary of all of the above

Older runs used `results_<mode>/` without the `_agg` / `_no_agg` suffix; `regenerate_report.sh`
still finds that layout as a fallback.

### Regenerate `report.md` only (no integration re-run)

From this directory, after you already have `raw/` + `crisp_verify_gas.json`:

```bash
./regenerate_report.sh --mode insecure
./regenerate_report.sh --mode insecure --no-proof-aggregation
```

`crisp_verify_gas.json` embeds the integration timings; if you also keep `integration_summary.json`
in the same folder, the script passes it explicitly (useful when gas JSON is missing a field but the
snapshot is complete). `regenerate_report.sh` itself does not re-run `test_trbfv_actor`; it renders
from the matching `results_<mode>_{agg|no_agg}/` directory.

## Refresh after parameter changes

If you change circuit/config parameter sets, rerun the full benchmark + gas extraction flow.

From repository root:

```bash
# Build CRISP SDK artifacts used by verifier tests
pnpm -C examples/CRISP/packages/crisp-sdk build

# Recompute benchmark raw JSON and base report (insecure mode)
./circuits/benchmarks/run_benchmarks.sh --mode insecure

# Extract on-chain verify gas from simulated verifier tests
./circuits/benchmarks/scripts/extract_crisp_verify_gas.sh \
  --output "./circuits/benchmarks/results_insecure_agg/crisp_verify_gas.json"

# Regenerate report with gas values
./circuits/benchmarks/scripts/generate_report.sh \
  --input-dir "./circuits/benchmarks/results_insecure_agg/raw" \
  --output "./circuits/benchmarks/results_insecure_agg/report.md" \
  --gas-json "./circuits/benchmarks/results_insecure_agg/crisp_verify_gas.json"
```

If Π_DKG / Π_dec **verify gas** is `N/A` because `crisp_verify_gas.json` came from a failed extract,
but your integration summary still has `folded_artifacts`, replay only the Hardhat `estimateGas`
step and merge **dkg** / **dec** into the gas file (no Rust re-run):

```bash
# For secure folded proofs, align Solidity verifiers first (--build may take a while).
./circuits/benchmarks/scripts/replay_folded_verify_gas.sh \
  --summary "/tmp/summary_secure.json" \
  --gas-json "./circuits/benchmarks/results_secure_agg/crisp_verify_gas.json" \
  --build secure-8192
```

If `crisp_verify_gas.json` has `integration_summary: null` but you still have the JSON written by
`BENCHMARK_SUMMARY_OUTPUT` from a successful `test_trbfv_actor` run (e.g.
`/tmp/summary_secure.json`), pass it so phase timings and folded sizes match that run:

```bash
./circuits/benchmarks/scripts/generate_report.sh \
  --input-dir "./circuits/benchmarks/results_secure_agg/raw" \
  --output "./circuits/benchmarks/results_secure_agg/report.md" \
  --gas-json "./circuits/benchmarks/results_secure_agg/crisp_verify_gas.json" \
  --integration-summary "/tmp/summary_secure.json"
```

For secure mode, use `--mode secure` and the `results_secure_{agg|no_agg}/` directories.

## Reported protocol tables

`results_*/report.md` now includes protocol-oriented sections in addition to raw category tables:

- `Circuit Benchmarks` with rows in fixed order: `C0`, `C1`, `C2a`, `C2b`, `C3a`, `C3b`, `C4a`,
  `C4b`, `C5`, `user-data-encryption`, `C6`, `C7`.
- `Artifacts` for `Π_DKG`, `Π_user`, `Π_dec` with proof/public-input sizes and gas columns.
- `Role / Phase / Activity` for P1..P4 operational cost summaries.
- When `integration_summary` is present, the report also includes:
  - an `Integration test` section (end-to-end phase wall-clock timings)
  - a `Thread pool` section (Rayon threads / cores)
  - `CPU-bound operation timings` (tracked in-process averages/totals)
  - `Proof aggregation / folding` (enabled flag, fold attestation verifier address)
  - `Aggregation / fold operation timings` (`ZkNodeDkgFold`, `ZkDkgAggregation`, etc.)
  - `Folded on-chain artifacts` (byte sizes used for Π_DKG / Π_dec gas replay)

## Derivation rules

- `Constraints`: `gates.total_gates`
- `Prove time (s)`: `proof_generation.time_seconds`
- `Verify time (ms)`: `verification.time_seconds * 1000`
- `Proof size (KB)`: `proof_generation.proof_size_bytes / 1024`
- `Public input size`: `verification.public_inputs_size_bytes / 1024`

Split rows are deterministic:

- `C3a` and `C3b` both map to `dkg/share_encryption` benchmark output.
- `C4a` and `C4b` both map to `dkg/share_decryption` benchmark output.

## Gas measurement source

`Verify gas` is sourced from the existing CRISP verification test path:

- Test: `examples/CRISP/packages/crisp-contracts/tests/crisp.contracts.test.ts`
- Command path: `circuits/benchmarks/scripts/extract_crisp_verify_gas.sh`
- Benchmark integration: `run_benchmarks.sh` runs that script and passes the JSON to report
  generation.

For `Π_DKG` and `Π_dec`, verifier gas is sourced from folded recursive-aggregation proofs exported
by `cargo test -p e3-tests test_trbfv_actor` (via `BENCHMARK_FOLDED_OUTPUT`) and then replayed into
EVM verifier `estimateGas` in `packages/enclave-contracts/scripts/benchmarkGasFromRaw.ts`.

`extract_crisp_verify_gas.sh` (and `replay_folded_verify_gas.sh --build <preset>`) call
`ensure_circuit_preset_built.sh`, which runs
`pnpm build:circuits --skip-if-built --no-clean --no-clean-targets` by default (skips recompile when
`dist/circuits/<preset>/.build-stamp.json` and marker artifacts match the current circuit sources).
Then `pnpm generate:verifiers --no-compile` refreshes Honk contracts before integration export and
Hardhat replay.

- **`--force-build`** on extract/replay/ensure: full rebuild (same as a fresh `build:circuits`).
- **`--skip-build`** on extract/replay: skip circuit build and Honk generation (only re-run
  integration + gas replay). Fails fast unless `dist/circuits/<preset>/` and `circuits/bin` targets
  are present for that preset (`check_circuit_preset_artifacts.sh`).

`run_benchmarks.sh` preflight uses the same `ensure` + `--skip-if-built`. When preset artifacts are
ready, per-circuit `nargo compile` is skipped automatically (Stage 1 `ensure` skips too). Generated
`Prover.toml` files under `circuits/bin/` are excluded from the preset source hash so benchmarks do
not invalidate the stamp. Use **`--bench-compile`** to force per-circuit compile timings anyway.

`Calldata gas` is computed from benchmark proof/public-input bytes with EVM calldata costs
(`0x00 -> 4`, non-zero byte -> 16) and stored in raw benchmark JSON.
