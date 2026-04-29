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

### What gets stored (secure / insecure)

A full `./run_benchmarks.sh --mode <mode>` run writes:

- `results_<mode>/raw/*.json` — Nargo timing + artifact sizes (source for the **Circuit Benchmarks**
  table)
- `results_<mode>/crisp_verify_gas.json` — verify gas, calldata gas, artifact sizes, and
  **`integration_summary`** from `test_trbfv_actor` when gas extraction succeeds
- `results_<mode>/integration_summary.json` — snapshot of `.integration_summary` (phase timings,
  folded proofs, **multithread / operation_timings** after a fresh integration export)
- `results_<mode>/report.md` — rendered summary of all of the above

### Regenerate `report.md` only (no integration re-run)

From this directory, after you already have `raw/` + `crisp_verify_gas.json`:

```bash
./regenerate_report.sh
./regenerate_report.sh --mode insecure
```

`crisp_verify_gas.json` embeds the integration timings; if you also keep `integration_summary.json`
in the same folder, the script passes it explicitly (useful when gas JSON is missing a field but the
snapshot is complete). `regenerate_report.sh` itself does not re-run `test_trbfv_actor`; it renders
from `results_<mode>/raw`, `crisp_verify_gas.json`, and (optionally) `integration_summary.json`.

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
  --output "./circuits/benchmarks/results_insecure/crisp_verify_gas.json"

# Regenerate report with gas values
./circuits/benchmarks/scripts/generate_report.sh \
  --input-dir "./circuits/benchmarks/results_insecure/raw" \
  --output "./circuits/benchmarks/results_insecure/report.md" \
  --gas-json "./circuits/benchmarks/results_insecure/crisp_verify_gas.json"
```

If Π_DKG / Π_dec **verify gas** is `N/A` because `crisp_verify_gas.json` came from a failed extract,
but your integration summary still has `folded_artifacts`, replay only the Hardhat `estimateGas`
step and merge **dkg** / **dec** into the gas file (no Rust re-run):

```bash
# For secure folded proofs, align Solidity verifiers first (--build may take a while).
./circuits/benchmarks/scripts/replay_folded_verify_gas.sh \
  --summary "/tmp/summary_secure.json" \
  --gas-json "./circuits/benchmarks/results_secure/crisp_verify_gas.json" \
  --build secure-8192
```

If `crisp_verify_gas.json` has `integration_summary: null` but you still have the JSON written by
`BENCHMARK_SUMMARY_OUTPUT` from a successful `test_trbfv_actor` run (e.g.
`/tmp/summary_secure.json`), pass it so phase timings and folded sizes match that run:

```bash
./circuits/benchmarks/scripts/generate_report.sh \
  --input-dir "./circuits/benchmarks/results_secure/raw" \
  --output "./circuits/benchmarks/results_secure/report.md" \
  --gas-json "./circuits/benchmarks/results_secure/crisp_verify_gas.json" \
  --integration-summary "/tmp/summary_secure.json"
```

For secure mode, use `--mode secure` and replace `results_insecure` with `results_secure`.

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

`Calldata gas` is computed from benchmark proof/public-input bytes with EVM calldata costs
(`0x00 -> 4`, non-zero byte -> 16) and stored in raw benchmark JSON.
