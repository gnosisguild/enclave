# zk-helpers

ZK circuit artifact generation for the Noir prover.

## zk-cli

Generate `Prover.toml` and `configs.nr` for a circuit.

```bash
# List circuits
cargo run -p e3-zk-helpers --bin zk_cli -- --list_circuits

# Generate artifacts (default: output/)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-bfv --preset default

# Custom output dir; skip Prover.toml (only configs.nr)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-bfv --preset default --output ./artifacts --toml
```

| Flag               | Description                                        |
| ------------------ | -------------------------------------------------- |
| `--list_circuits`  | List circuits and exit                             |
| `--circuit <name>` | Circuit (e.g. `pk-bfv`)                            |
| `--preset <name>`  | BFV preset (must match circuit)                    |
| `--output <path>`  | Output dir (default: `output`)                     |
| `--toml`           | Skip writing Prover.toml; always writes configs.nr |
