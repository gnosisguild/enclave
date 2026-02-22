# Circuit benchmarks

Benchmark the ZK circuits. Configuration is in `config.json` (circuit list, mode, oracles, metrics).

## How to run

From the **benchmarks** directory:

```bash
./run_benchmarks.sh [options]
```

Or from **scripts**:

```bash
./scripts/run_benchmarks.sh [options]
```

Both use `config.json` in the benchmarks directory by default.

### Options

| Option                      | Description                                                                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `--mode insecure \| secure` | Run in insecure (default) or secure mode. Overrides `config.json`’s `mode`.                                 |
| `--circuit <path>`          | Run only this circuit (e.g. `dkg/pk` or `config`). If not in `config.json`, runs anyway if the path exists. |
| `--config <file>`           | Use a different config file instead of `config.json`.                                                       |
| `--skip-compile`            | Reuse existing build artifacts; skip compilation.                                                           |
| `--clean`                   | Remove circuit `target/` directories after the run.                                                         |

### Examples

```bash
# Default: insecure mode, all circuits from config (config circuit is skipped)
./run_benchmarks.sh

# Secure mode (includes the config circuit)
./run_benchmarks.sh --mode secure

# Single circuit
./run_benchmarks.sh --circuit threshold/pk_generation
./run_benchmarks.sh --mode secure --circuit config

# Re-run without recompiling
./run_benchmarks.sh --skip-compile
```

### Results

Output goes under `results_<mode>/` (e.g. `results_insecure/`, `results_secure/`). A Markdown report
is written to `results_<mode>/report.md`. Raw JSON is kept in `results_<mode>/raw/` so that a run
with `--circuit` only overwrites that circuit’s file and the report is regenerated from all data
(existing + updated). View the report with `cat results_<mode>/report.md` or
`open results_<mode>/report.md` (macOS).

## Secure-only circuits

The **config** circuit (validates secure configs only) is listed in `config.json` with
`"modes": ["secure"]` so it is only run in secure mode. With the default `"mode": "insecure"` it is
skipped. The script `scripts/run_benchmarks.sh` respects this by filtering circuits by the `modes`
field in `config.json` before running; see the “Circuit-specific modes” comment and the loop that
builds `RUN_CIRCUITS` there.
