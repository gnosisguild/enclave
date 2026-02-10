# zk-helpers

ZK circuit artifact generation for the Noir prover. Produces `configs.nr` and optionally
`Prover.toml` for the Enclave circuits.

## zk-cli

Generate `configs.nr` for a circuit; use `--toml` to also generate `Prover.toml`.

```bash
# List circuits
cargo run -p e3-zk-helpers --bin zk_cli -- --list_circuits

# Generate configs.nr only (default)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure

# Generate configs.nr and Prover.toml (--witness required for share-computation)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure --toml
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit share-computation --preset insecure --witness secret-key --toml

# Generate only Prover.toml (no configs.nr), e.g. for benchmarks where circuits use lib configs
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk --preset insecure --toml --no-configs
```

| Flag               | Description                                                                    |
| ------------------ | ------------------------------------------------------------------------------ |
| `--list_circuits`  | List circuits and exit                                                         |
| `--circuit <name>` | Circuit name (e.g. `pk`, `share-computation`, `threshold-share-decryption`)    |
| `--preset <name>`  | Security preset: `insecure` (512) or `secure` (8192)                           |
| `--witness <type>` | For DKG witness circuits when using `--toml`: `secret-key` or `smudging-noise` |
| `--output <path>`  | Output dir (default: `output`)                                                 |
| `--toml`           | Also write Prover.toml (default: configs.nr only)                              |
| `--no-configs`     | With `--toml`: do not write configs.nr (e.g. for circuit benchmarks)           |
