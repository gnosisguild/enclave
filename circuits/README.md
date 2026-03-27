# Circuits

This directory holds the **Noir** implementation of Interfold’s zero-knowledge circuits: distributed
key generation and encrypted share handling (**BFV**), threshold key generation, user encryption,
and threshold decryption (**TrBFV**), together with recursive proof aggregation.

The Noir sources and tests in this tree are authoritative for constraints and public inputs.
Everything else—docs, diagrams, comments—is there to help you navigate; when in doubt, trust the
code.

```text
circuits/
├── lib/
│   └── src/
│       ├── configs/           # BFV / CRT parameter presets
│       ├── core/dkg/          # Shared logic: C0, C2–C4
│       ├── core/threshold/    # Shared logic: C1, C5, P3, C6, C7
│       └── math/              # Polynomials, SAFE, commitments, modular arithmetic
├── bin/
│   ├── config/                # Deployment-time consistency checks on presets
│   ├── dkg/                   # DKG packages and C2 proof pipeline
│   ├── threshold/             # TrBFV, user encryption, threshold decryption
│   └── recursive_aggregation/
│       ├── fold/
│       └── wrapper/
│           ├── dkg/
│           └── threshold/
└── benchmarks/
```

The shared library is the Nargo package **`lib`** (`lib/Nargo.toml`). All packages under `bin/`
depend on it; module structure is documented in [`lib/src/README.md`](lib/src/README.md).

Packages under `bin/` with a `Nargo.toml` are build targets. Directory names align with the
**`CircuitName`** enum in `crates/events` via `CircuitName::group()` and `CircuitName::dir_path()`.
Workspace manifests also exist at `dkg/` and `threshold/` for grouped builds.

## Circuit package index

The tables below map **`circuits/bin/` paths** to **circuit labels** (C0–C7) and **`CircuitName`**
values used in Rust. Phases **P1–P4** are a product-level grouping of the same protocol steps; for
how phases, commitments, and circuit IDs line up end to end, read
[Cryptography](https://docs.theinterfold.com/cryptography) (source:
[`docs/pages/cryptography.mdx`](../docs/pages/cryptography.mdx)).

**C2** is implemented as a **pipeline** of packages (base, chunk, batch, final `share_computation`),
not a single crate.

### DKG (`bin/dkg/`)

| Path                            | ID       | `CircuitName`                | Role                                          |
| ------------------------------- | -------- | ---------------------------- | --------------------------------------------- |
| `pk`                            | C0       | `PkBfv`                      | Commit to individual BFV public key           |
| `sk_share_computation_base`     | C2 inner | `SkShareComputationBase`     | Shamir tensor for secret contribution         |
| `e_sm_share_computation_base`   | C2 inner | `ESmShareComputationBase`    | Shamir tensor for smudging noise              |
| `share_computation_chunk`       | C2 inner | `ShareComputationChunk`      | Reed–Solomon parity on a coefficient slice    |
| `share_computation_chunk_batch` | C2 inner | `ShareComputationChunkBatch` | Binds base proof to a batch of chunk proofs   |
| `share_computation`             | **C2**   | `ShareComputation`           | Final C2 step; aggregates inner proofs        |
| `share_encryption`              | C3       | `ShareEncryption`            | BFV encryption of shares under recipient keys |
| `share_decryption`              | C4       | `DkgShareDecryption`         | Decrypt shares; aggregate; commitments for P4 |

### Threshold (`bin/threshold/`)

| Path                           | ID         | `CircuitName`                | Role                                              |
| ------------------------------ | ---------- | ---------------------------- | ------------------------------------------------- |
| `pk_generation`                | C1         | `PkGeneration`               | Threshold public-key contribution                 |
| `pk_aggregation`               | C5         | `PkAggregation`              | Aggregate contributions into threshold public key |
| `user_data_encryption_ct0`     | P3         | —                            | User ciphertext (first leg)                       |
| `user_data_encryption_ct1`     | P3         | —                            | User ciphertext (second leg)                      |
| `user_data_encryption`         | P3 wrapper | —                            | Wrapper: ct0, ct1, shared randomness              |
| `share_decryption`             | C6         | `ThresholdShareDecryption`   | Partial decryption share                          |
| `decrypted_shares_aggregation` | C7         | `DecryptedSharesAggregation` | Combine shares; CRT; decode                       |

### Recursive aggregation (`bin/recursive_aggregation/`)

| Path                  | `CircuitName` | Role                                                      |
| --------------------- | ------------- | --------------------------------------------------------- |
| `fold`                | `Fold`        | Fold two wrapper outputs                                  |
| `wrapper/dkg/*`       | —             | Verifies inner DKG proofs; compresses public inputs       |
| `wrapper/threshold/*` | —             | Verifies inner threshold proofs; compresses public inputs |

Wrapper parameters are documented in
[`wrapper/README.md`](bin/recursive_aggregation/wrapper/README.md).

### Configuration

| Path     | Role                                                                    |
| -------- | ----------------------------------------------------------------------- |
| `config` | Validates secure preset constants (CRT moduli, bounds, parity matrices) |

### Per-package READMEs (`bin/**/README.md`)

Many packages include a **short** README for navigation in the file tree. Keep them that way: one or
two sentences on what this binary proves, then a small table—**do not** duplicate
[Cryptography](https://docs.theinterfold.com/cryptography) or the full package index.

| Row | Purpose |
| --- | ------- |
| **Core** (or **Source**) | Link to the shared implementation in [`lib/src/`](lib/src/README.md), or to [`src/main.nr`](bin/dkg/pk/src/main.nr) when the crate is self-contained. |
| **Index** | Link to [**Circuit package index**](#circuit-package-index) in this file. Use the right number of `../` so the path reaches `circuits/README.md` (depends on folder depth; all current READMEs are already consistent). |
| **Docs** | [Noir Circuits](https://docs.theinterfold.com/noir-circuits) for toolchain and layout, plus the repo [source](../docs/pages/noir-circuits.mdx). |

Optional extras (only when they save a click): **Wrappers** (recursive aggregation), or a second
sentence naming the paired package (e.g. P3 `ct0` / `ct1`).

## Build and test

From the repository root:

```bash
pnpm tsx scripts/build-circuits.ts   # compile circuits, verification keys, artifacts
./scripts/lint-circuits.sh           # nargo fmt --check; nargo check (skipped if nargo absent)
./scripts/test-circuits.sh           # unit tests in circuits/lib
```

Pin **nargo** and **bb** to the versions in `crates/zk-prover` and `versions.json`. For local work,
**`enclave noir setup`** installs a toolchain that lines up with the prover and the artifacts CI
produces. Install options and CLI flags are on the
[Noir Circuits](https://docs.theinterfold.com/noir-circuits) page
([`docs/pages/noir-circuits.mdx`](../docs/pages/noir-circuits.mdx)).

## Related documentation

| Topic                                                                  | Location                                                                                                 |
| ---------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Cryptographic model (PV-TBFV, phases P1–P4, circuit identifiers C0–C7) | [Cryptography](https://docs.theinterfold.com/cryptography) · [source](../docs/pages/cryptography.mdx)    |
| Toolchain, repository layout, `enclave noir`, compilation              | [Noir Circuits](https://docs.theinterfold.com/noir-circuits) · [source](../docs/pages/noir-circuits.mdx) |
| Rust types (`ProofType`, `CircuitName`)                                | [`signed_proof.rs`](../crates/events/src/enclave_event/signed_proof.rs) · [`proof.rs`](../crates/events/src/enclave_event/proof.rs) |
| Protocol execution (actors, events, proof ordering)                    | [`agent/flow-trace/04_DKG_AND_COMPUTATION.md`](../agent/flow-trace/04_DKG_AND_COMPUTATION.md)            |
