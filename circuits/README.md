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

Packages under `bin/` with a `Nargo.toml` are build targets unless they contain a `DISABLED` marker.
The marker retains the historical source and explains why canonical discovery and verifier
generation exclude it; explicitly requesting a disabled circuit fails. Directory names align with
the **`CircuitName`** enum in `crates/events` via `CircuitName::group()` and
`CircuitName::dir_path()`. Workspace manifests also exist at `dkg/` and `threshold/` for grouped
builds.

## Circuit package index

The tables below map **`circuits/bin/` paths** to **circuit labels** (C0–C7) and **`CircuitName`**
values used in Rust. Phases **P1–P4** are a product-level grouping of the same protocol steps; for
how phases, commitments, and circuit IDs line up end to end, read
[Cryptography](https://docs.theinterfold.com/learn/cryptography) (source:
[`docs/pages/learn/cryptography.mdx`](../docs/pages/learn/cryptography.mdx)).

**C2a** proves the SK shares with the packed PRF-key transport. The current `node_fold` verifies
C0/C1/C2a, the C3 chain and C4a directly. The old shared-smudging C2b and paired C2ab/C3ab/C4ab
folds are **disabled, not deleted**: their sources remain, but they are absent from new canonical
builds, the required installation inventory and active benchmarks. The DKG workspace excludes C2b,
and lint skips marked packages. Historical artifacts, reports and durable enum IDs remain intact.
Fresh local smudging noise is still required and proved by **C6**; it is not the retired DKG track.

### DKG (`bin/dkg/`)

| Path                     | ID  | `CircuitName`         | Role                                           |
| ------------------------ | --- | --------------------- | ---------------------------------------------- |
| `pk`                     | C0  | `PkBfv`               | Commit to individual BFV public key            |
| `sk_share_computation`   | C2a | `SkShareComputation`  | Secret-key track Shamir shares (`y`)           |
| `e_sm_share_computation` | C2b | `ESmShareComputation` | Disabled historical shared-noise Shamir shares |
| `share_encryption`       | C3  | `ShareEncryption`     | BFV encryption of shares under recipient keys  |
| `share_decryption`       | C4  | `DkgShareDecryption`  | Decrypt shares; aggregate; commitments for P4  |

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

| Path                                  | `CircuitName`                  | Role                                         |
| ------------------------------------- | ------------------------------ | -------------------------------------------- |
| `c3_fold`, `c3_fold_kernel`           | `C3Fold`, `C3FoldKernel`       | Fold the selected SK/PRF C3 inputs           |
| `node_fold`                           | `NodeFold`                     | Bind one node's C0/C1/C2a/C3/C4a inputs      |
| `nodes_fold`, `nodes_fold_kernel`     | `NodesFold`, `NodesFoldKernel` | Fold the selected H node proofs              |
| `c6_fold`, `c6_fold_kernel`           | `C6Fold`, `C6FoldKernel`       | Fold C6 shares bound to one synchronized set |
| `dkg_aggregator`                      | `DkgAggregator`                | Verify the DKG fold and C5                   |
| `decryption_aggregator`               | `DecryptionAggregator`         | Verify the C6 fold and C7                    |
| `c2ab_fold`, `c3ab_fold`, `c4ab_fold` | retained legacy variants       | Disabled historical paired SK/ESM folds      |

For recursive trust and public-input bindings, see
[proof statements](../docs/architecture/proof-statements.md#recursive-trust-and-exported-values).

### Configuration

| Path     | Role                                                                    |
| -------- | ----------------------------------------------------------------------- |
| `config` | Validates secure preset constants (CRT moduli, bounds, parity matrices) |

## Build and test

From the repository root:

```bash
pnpm build:circuits                # compile circuits, verification keys, artifacts
pnpm noir:lint                    # nargo fmt --check; nargo check (skipped if nargo absent)
pnpm noir:test                    # nargo test: lib, decryption_aggregator, dkg_aggregator and node_fold (3 preset/committee pairs); nargo execute: config
```

Pin **nargo** and **bb** to the versions in `crates/zk/prover` and `versions.json`. For local work,
**`interfold noir setup`** installs a toolchain that lines up with the prover and the artifacts CI
produces. Install options and CLI flags are on the
[Noir Circuits](https://docs.theinterfold.com/build/noir-circuits) page
([`docs/pages/build/noir-circuits.mdx`](../docs/pages/build/noir-circuits.mdx)).

## Related documentation

| Topic                                                                  | Location                                                                                                                                |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Cryptographic model (PV-TBFV, phases P1–P4, circuit identifiers C0–C7) | [Cryptography](https://docs.theinterfold.com/learn/cryptography) · [source](../docs/pages/learn/cryptography.mdx)                       |
| Toolchain, repository layout, `interfold noir`, compilation            | [Noir Circuits](https://docs.theinterfold.com/build/noir-circuits) · [source](../docs/pages/build/noir-circuits.mdx)                    |
| Rust types (`ProofType`, `CircuitName`)                                | [`signed_proof.rs`](../crates/events/src/interfold_event/signed_proof.rs) · [`proof.rs`](../crates/events/src/interfold_event/proof.rs) |
| Protocol execution (actors, events, proof ordering)                    | [DKG](../docs/architecture/dkg.md) and [threshold decryption](../docs/architecture/threshold-decryption.md)                             |
