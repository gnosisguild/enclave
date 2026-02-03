# zk-helpers

ZK circuit artifact generation for the Noir prover. Produces `configs.nr` and optionally `Prover.toml` for the pk-bfv and share-computation circuits.

## zk-cli

Generate `Prover.toml` and `configs.nr` for a circuit.

```bash
# List circuits
cargo run -p e3-zk-helpers --bin zk_cli -- --list_circuits

# Generate artifacts for pk-bfv (default output: output/)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-bfv --preset insecure

# Generate artifacts for share-computation (--witness required when writing Prover.toml)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --witness secret-key
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset secure --witness smudging-noise

# Configs only (no Prover.toml): --witness optional for share-computation (configs are shared)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-bfv --preset insecure --output ./artifacts --toml
```

| Flag               | Description                                                                                               |
| ------------------ | --------------------------------------------------------------------------------------------------------- |
| `--list_circuits`  | List circuits and exit                                                                                    |
| `--circuit <name>` | Circuit: `pk-bfv` or `share-computation`                                                                  |
| `--preset <name>`  | Security preset: `insecure` (512) or `secure` (8192)                                                      |
| `--witness <type>` | For `share-computation` when writing Prover.toml: `secret-key` or `smudging-noise` (optional if `--toml`) |
| `--output <path>`  | Output dir (default: `output`)                                                                            |
| `--toml`           | Skip writing Prover.toml; always writes configs.nr                                                        |
