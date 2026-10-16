# e3-zk-artifacts

## Purpose

The `zk-artifacts` executable generates Interfold circuit artifacts. Its commands call the
computation, encoding, and commitment libraries. Argument parsing, command dispatch, and terminal
output belong to this package.

## Map

| Location                                       | Responsibility                                                               |
| ---------------------------------------------- | ---------------------------------------------------------------------------- |
| [`main.rs`](src/main.rs)                       | Clap options, command dispatch, metadata listing, and terminal output.       |
| [`generate.rs`](src/generate.rs)               | Circuit/preset/input-family dispatch into computation/sample/codegen traits. |
| [`vk_hash.rs`](src/vk_hash.rs)                 | Canonical field parsing and complete recursive VK-tree combination.          |
| [`parity_matrices.rs`](src/parity_matrices.rs) | Committee-specific parity derivation and Noir file output.                   |

Generation resolves circuit metadata, samples inputs, computes bounds/witnesses, and writes the
selected artifacts. Its samples are not production request inputs; `Prover.toml` includes private
witness data. The shared implementation lives in [zk-helpers](../../zk-helpers/README.md), not in
the CLI's dispatch table.

## Commands

Run these commands from the repository root:

```bash
pnpm zk list

pnpm zk generate --circuit pk-generation --preset insecure --committee minimum --toml

pnpm zk generate --circuit share-encryption --preset insecure --committee micro --inputs secret-key --toml --no-configs

pnpm zk vk-hash path/to/first.vk_hash path/to/second.vk_hash

pnpm zk vk-hash --bfv-tree path/to/artifact-pair

pnpm zk parity-matrices --committee minimum
```

`generate` always samples circuit data and computes both artifacts. By default, it writes only
`configs.nr` to `output/`. Use `--toml` to also write `Prover.toml`. With `--toml --no-configs`, it
writes only `Prover.toml`. Use `--output <path>` to select another directory.

`--preset` accepts `insecure` (degree 512), `secure` (degree 8192), or aliases `2` and `80`.
`--committee` accepts `minimum` (default), `micro`, or `small`. The committee must match the
compiled Noir circuits.

When you request `Prover.toml`, `share-computation`, `share-encryption`, and `share-decryption`
require `--inputs secret-key|smudging-noise`. Config-only generation defaults to `secret-key` for
these circuits.

`vk-hash` preserves input order and rejects files outside the canonical 32-byte field encoding.
`--bfv-tree` reads the complete recursive key tree and prints the `nodes_fold` and `c6_fold` hashes
as JSON.

`parity-matrices` writes both preset files under `circuits/lib/src/configs/committee/<name>/`. Use
`--output-root <path>` to select another root that contains the committee directory. To change the
active committee or preset, use `pnpm build:circuits`, not direct edits to generated files.

## Tests

Run `cargo test --locked -p e3-zk-artifacts` from the root. Inline `vk_hash.rs` tests check
full-tree descendant ordering, missing keys, and noncanonical fields using local artifacts.
Computation and artifact-writer tests live in [zk-helpers](../../zk-helpers/README.md#tests).
Running generation samples data/writes files but does not prove a circuit; the prepared proof flow
is in [circuit artifacts](../../../docs/architecture/circuit-artifacts.md). See
[AGENTS.md](AGENTS.md) for dispatch/generated-output changes.
