# e3-zk-artifacts

The `zk-artifacts` executable generates Interfold circuit artifacts. Its commands call the
computation, encoding, and commitment libraries. Argument parsing, command dispatch, and terminal
output belong to this package.

## Commands

Run these commands from the repository root:

```bash
pnpm zk list

pnpm zk generate --circuit pk-generation --preset insecure --committee minimum --toml

pnpm zk generate --circuit share-encryption --preset insecure --committee micro --inputs secret-key --toml --no-configs

pnpm zk generate --circuit share-computation --preset secure-8192 --inputs secret-key --chunk-size 512

pnpm zk generate --circuit rlk-generation --preset secure-16384 --row-index 0

pnpm zk vk-hash path/to/first.vk_hash path/to/second.vk_hash

pnpm zk vk-hash --bfv-tree path/to/artifact-pair

pnpm zk vk-hash --bfv-tree path/to/artifact-pair --decryption-tree-only

pnpm zk parity-matrices --committee minimum

pnpm zk config-modules --preset SECURE_THRESHOLD_8192
```

`generate` always samples circuit data and computes both artifacts. By default, it writes only
`configs.nr` to `output/`. Use `--toml` to also write `Prover.toml`. With `--toml --no-configs`, it
writes only `Prover.toml`. Use `--output <path>` to select another directory.

`--preset` accepts a preset name (for example, `INSECURE_THRESHOLD`, `SECURE_THRESHOLD_8192`, or
`SECURE_THRESHOLD_16384`), an alias (`insecure`, `insecure-lbfv`, `secure-8192`, or
`secure-16384`), or a security level.
`--committee` accepts `minimum` (default), `micro`, or `small`. The committee must match the
compiled Noir circuits.

`--chunk-size <n>` sets the C2 coefficient chunk size. The default is the compiled size for the
preset and committee. The value must divide the polynomial degree. `--row-index <n>` selects the
row of the public l-BFV key-switching vectors for the l-BFV circuits. The default is `0`.

When you request `Prover.toml`, `share-computation`, `share-encryption`, and `share-decryption`
require `--inputs secret-key|smudging-noise`. Config-only generation defaults to `secret-key` for
these circuits.

`vk-hash` preserves input order and rejects files outside the canonical 32-byte field encoding.
`--bfv-tree` reads the complete recursive key tree and prints the `nodes_fold` and `c6_fold` hashes
as JSON. With `--decryption-tree-only`, it reads only the C6 tree and prints only `c6_fold`. Use this
flag for an l-BFV-only pair (secure-16384), which has no trBFV DKG tree.

`parity-matrices` writes the parity matrix file of each preset and the `smudging.nr` file under
`circuits/lib/src/configs/committee/<name>/`. Use
`--output-root <path>` to select another root that contains the committee directory. To change the
active committee or preset, use `pnpm build:circuits`, not direct edits to generated files.

`config-modules --preset <name>` writes `mod.nr`, `threshold.nr`, and `dkg.nr` (and the `lbfv/`
modules when the preset enables l-BFV) under `circuits/lib/src/configs/<preset-module>/`. The preset
must be a threshold preset. Use `--output-root <path>` to select another configs root.
