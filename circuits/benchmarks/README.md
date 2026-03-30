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
