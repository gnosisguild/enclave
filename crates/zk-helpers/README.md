# zk-helpers

ZK circuit artifact generation for the Noir prover. Produces `configs.nr` and optionally
`Prover.toml` for the Interfold circuits. The Prover.toml contains circuit inputs for Nargo, which
executes them to produce witnesses for proof generation.

## zk-cli

Generate `configs.nr` for a circuit; use `--toml` to also generate `Prover.toml`.

```bash
# List circuits
cargo run -p e3-zk-helpers --bin zk_cli -- --list_circuits

# Generate configs.nr only (default; --committee defaults to micro)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --inputs secret-key
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --inputs smudging-noise

# Medium or large committee (must match active circuits lib selection)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-generation --preset insecure --committee medium
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-generation --preset insecure --committee large

# Generate configs.nr and Prover.toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --inputs secret-key --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --inputs smudging-noise --toml

# Generate only Prover.toml (no configs.nr), e.g. for benchmarks where circuits use lib configs
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure --toml --no-configs
```

| Flag                 | Description                                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------------------------- |
| `--list_circuits`    | List circuits and exit                                                                                        |
| `--circuit <name>`   | Circuit name (e.g. `pk`, `share-computation`)                                                                 |
| `--preset <name>`    | Security preset: `insecure` (512) or `secure` (8192)                                                          |
| `--committee <name>` | Committee size: `micro` (default), `small`, `medium`, or `large` — must match `circuits/lib` active committee |
| `--inputs <type>`    | Select the witness family when sample generation depends on it: `secret-key` or `smudging-noise`              |
| `--output <path>`    | Output dir (default: `output`)                                                                                |
| `--toml`             | Also write Prover.toml (default: configs.nr only)                                                             |
| `--no-configs`       | With `--toml`: do not write configs.nr (e.g. for circuit benchmarks)                                          |
