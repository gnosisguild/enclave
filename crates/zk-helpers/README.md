# e3-zk-helpers

## Purpose

ZK circuit computation, witness encoding, commitments, and artifact generation shared by native
clients, ciphernode proof preparation, and the artifact CLI. This crate computes circuit inputs;
[zk-prover](../zk-prover/README.md) executes witnesses/proofs.

Polynomial arithmetic belongs to `e3-polynomial`; FHE conversion belongs to `e3-polynomial-fhe`.
Circuit-specific reversal, centering, packing, and JSON encoding stay in this crate. The
crate-private `negacyclic` module owns descending witness folding and its independent reference
tests.

## Map

| Location                                                                                                               | Responsibility                                                                |
| ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [`metadata.rs`](src/metadata.rs), [`committee.rs`](src/committee.rs)                                                   | Circuit identity/parameter-family trait and compiled committee compatibility. |
| [`circuits/computation.rs`](src/circuits/computation.rs), [`circuits/codegen.rs`](src/circuits/codegen.rs)             | Shared computation/code-generation interfaces and artifact writers.           |
| [`circuits/dkg/`](src/circuits/dkg), [`circuits/threshold/`](src/circuits/threshold)                                   | Capability-specific data, computation, sample, and codegen modules.           |
| [`encoding.rs`](src/encoding.rs), [`packing.rs`](src/packing.rs), [`negacyclic.rs`](src/negacyclic.rs)                 | Signed-field conversion, coefficient packing, and exact witness folding.      |
| [`circuits/commitments.rs`](src/circuits/commitments.rs), [`circuits/output_layout.rs`](src/circuits/output_layout.rs) | Circuit-compatible commitments and public-input/output extraction.            |

Each circuit combines metadata and preset/committee data with a computation implementation. Samples
provide synthetic data for tooling; codegen emits Noir configuration and private witness files.
Runtime callers use the same computations on real request inputs without sampling.

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

## Library entry points

The metadata trait is `e3_zk_helpers::metadata::Circuit` (also re-exported at the crate root).
`Computation::compute(preset, &data)` produces a typed result with `to_json`; `CircuitComputation`
associates a circuit's metadata with its computation output. `CircuitCodegen` and the artifact
writer functions expose configuration/TOML generation. Concrete types live below `circuits::dkg` or
`circuits::threshold`; see the [user-encryption caller](../bfv-client/src/client.rs) for a real
input computation and the [CLI dispatcher](../zk/cli/src/generate.rs) for sample-driven artifact
generation.

## Tests

Run `cargo test --locked -p e3-zk-helpers` from the root. Inline circuit suites cover computation
identities, witness shapes, and generated TOML/config content; encoding/packing tests pin boundary
values and reference equivalence. Negacyclic folding compares an independent division reference.
These tests do not invoke Noir/Barretenberg proving; matching prepared proofs are a separate
[artifact workflow](../../docs/architecture/circuit-artifacts.md). See [AGENTS.md](AGENTS.md) for
ownership and witness-ABI changes.
