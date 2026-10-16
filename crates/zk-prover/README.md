# e3-zk-prover

Noir witness/proof execution, verification, recursive folding, and artifact installation for
ciphernode protocol proofs. This is separate from the
[OpenVM computation worker](../openvm-prover/README.md). Circuit-specific arithmetic/witness shapes
belong in [e3-zk-helpers](../zk-helpers/README.md).

## Backend and public API

`ZkBackend::new(bb_path, circuits_dir, work_dir)` takes `e3_config::BBPath` and artifact/work paths.
`check_status().await` reports setup state; `ensure_installed().await` prepares required
tools/artifacts and may download/install them. Version/checksum requirements come from
[versions.json](versions.json). `with_bb_timeout` bounds each proving process.

`ZkProver::new(&backend)` executes proof generation and verification. Select the matching artifact
directory with `resolve_artifacts_dir(preset, committee)` and use the explicit variant methods where
needed. Verification returns `Result<bool, ZkError>`: infrastructure failure is distinct from
completed invalid verification. `CompiledCircuit`, `WitnessGenerator`, and `input_map` expose
compiled-circuit loading and witness generation (using `noirc_abi::InputMap`) without defining
another circuit parameter table.

Node runtimes use `setup_zk_actors` with `ZkActorRecovery`, canonical committees/keys, signer, and
repositories. The [builder wiring](../ciphernode-builder/src/builder/extensions.rs) shows hydration
before effect-enabled replay. Operators normally use the [CLI Noir commands](../cli/src/noir.rs),
not manual actor construction. Slashing implementation lives in
[e3-slashing](../slashing/README.md).

## Before running Cargo

`build.rs` runs `scripts/build_fixtures.sh` even for checks. With nargo and required artifacts
available, it compiles the dummy Noir fixture and rewrites tracked fixture JSON using jq. If
required circuit artifacts are absent and bb is available, it can run
`pnpm install --frozen-lockfile` and `pnpm build:circuits` in a separate target directory. Missing
nargo/bb can skip preparation; a successful Cargo command does not establish matching artifacts.

For **compile-only** work:

```sh
E3_ZK_PROVER_SKIP_FIXTURE_BUILD=1 cargo check --locked -p e3-zk-prover --tests
```

This skips fixture preparation only. It neither skips proof tests nor supplies valid artifacts. The
dependency graph can also need [Solidity fixtures](../test-helpers/README.md).

## Real-proof tests

Prepare pinned nargo and bb, jq, pnpm/workspace dependencies, and circuits/VKs matching the active
preset and committee through the [circuit build guide](../../scripts/README.md#circuit-builder).
Then use root `pnpm rust:test:proofs` for the explicitly selected ignored proof suites, or
`pnpm rust:test:slashing` for proof-to-slash integration. The `integration-tests` feature gates the
separate network/download integration target; enabling it alone is not evidence of proof execution.
Test-only aggregation skipping in the wider harness is never a production fallback.

See [proof statements](../../docs/architecture/proof-statements.md) and
[artifact compatibility](../../docs/architecture/circuit-artifacts.md) for required cross-layer
acceptance. Witness generation, process success, checksum matching, and verified statements are
different results.
