# e3-zk-helpers

ZK circuit computation, witness encoding, commitments, and artifact generation.

Polynomial arithmetic belongs to `e3-polynomial`; FHE conversion belongs to `e3-polynomial-fhe`.
Circuit-specific reversal, centering, packing, and JSON encoding stay in this crate. The
crate-private `negacyclic` module owns descending witness folding and its independent reference
tests.

## Ownership

| Owner                    | Responsibility                                                        |
| ------------------------ | --------------------------------------------------------------------- |
| `e3-committee`           | Committee types and canonical parameter values                        |
| `e3_fhe_params::utils`   | Centered Q residues, k0_i batch inverses, and coefficient widths      |
| `e3-polynomial`          | Exact coefficient arithmetic and ordered CRT representations          |
| `e3-polynomial-fhe`      | Conversion between core polynomials and FHE representations           |
| `encoding`               | Canonical signed-field values and JSON witness shapes                 |
| `committee`              | Compatibility with the compiled circuit committee                     |
| `metadata`               | Circuit names, prefixes, parameter types, and witness families        |
| `packing` and `circuits` | Noir packing, commitments, computation, sampling, and code generation |

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
