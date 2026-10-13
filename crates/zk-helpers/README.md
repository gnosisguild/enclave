# e3-zk-helpers

ZK circuit helpers and artifact generation. `e3-committee` owns shared committee types and canonical
parameter values. The public-key aggregation workflow owns honest-roster selection.

Polynomial arithmetic belongs to `e3-polynomial`; FHE conversion belongs to `e3-polynomial-fhe`.
Circuit-specific reversal, centering, packing, and JSON encoding stay in this crate. The
crate-private `negacyclic` module owns descending witness folding and its independent reference
tests.

`e3_fhe_params::utils` owns centered Q residues, k0_i inverses, and coefficient widths. Plaintext
decoding calls `fhe_traits::FheDecoder` directly.

## zk_cli

```bash
# List circuits
cargo run -p e3-zk-helpers --bin zk_cli -- --list_circuits

# Generate Prover.toml + configs.nr for a circuit
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-generation --preset insecure --committee minimum

# Micro or small committee (must match active circuits lib selection)
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-generation --preset insecure --committee micro
cargo run -p e3-zk-helpers --bin zk_cli -- --circuit pk-generation --preset insecure --committee small
```

| Flag                 | Description                                                                                           |
| -------------------- | ----------------------------------------------------------------------------------------------------- |
| `--circuit <name>`   | Circuit to generate artifacts for                                                                     |
| `--preset <name>`    | BFV preset: `insecure` (512), `secure` (8192), or aliases `2` / `80`                                  |
| `--committee <name>` | Committee size: `minimum` (default), `micro`, or `small` — must match `circuits/lib` active committee |
