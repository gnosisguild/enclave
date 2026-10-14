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

For artifact commands, see [`e3-zk-artifacts`](../zk/cli/README.md). Run `pnpm zk --help` from the
repository root.
