# zk-helpers

ZK circuit artifact generation for the Noir prover. Produces `configs.nr` and optionally
`Prover.toml` for the Enclave circuits. The Prover.toml contains circuit inputs for Nargo, which
executes them to produce witnesses for proof generation.

## zk-cli

Generate `configs.nr` for a circuit; use `--toml` to also generate `Prover.toml`.

```bash
# List circuits
cargo run -p e3-zk-helpers --bin zk_cli -- --list_circuits

# Generate configs.nr only (default)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation-base --preset insecure --inputs secret-key
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation-base --preset insecure --inputs smudging-noise
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation-chunk --preset secure --inputs secret-key --chunk-idx 1

# Generate configs.nr and Prover.toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation-base --preset insecure --inputs secret-key --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation-base --preset insecure --inputs smudging-noise --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation-chunk --preset secure --inputs smudging-noise --chunk-idx 2 --toml

# Generate only Prover.toml (no configs.nr), e.g. for benchmarks where circuits use lib configs
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure --toml --no-configs
```

| Flag               | Description                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------------ |
| `--list_circuits`  | List circuits and exit                                                                           |
| `--circuit <name>` | Circuit name (e.g. `pk`, `share-computation-base`, `share-computation-chunk`)                    |
| `--preset <name>`  | Security preset: `insecure` (512) or `secure` (8192)                                             |
| `--inputs <type>`  | Select the witness family when sample generation depends on it: `secret-key` or `smudging-noise` |
| `--chunk-idx <n>`  | For `share-computation-chunk`: select which `y` slice to export as `y_chunk`                     |
| `--output <path>`  | Output dir (default: `output`)                                                                   |
| `--toml`           | Also write Prover.toml (default: configs.nr only)                                                |
| `--no-configs`     | With `--toml`: do not write configs.nr (e.g. for circuit benchmarks)                             |
