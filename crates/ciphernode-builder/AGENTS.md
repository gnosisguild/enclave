# e3-ciphernode-builder

Read this file before you change this crate. `agent/RULES.md` still applies.

## Purpose

This crate is the composition root of a ciphernode. `CiphernodeBuilder` opens and checks the stores,
connects the chains, attaches the actors, starts the network, and waits for startup sync. Then it
returns a `CiphernodeHandle`. `e3-entrypoint` builds production nodes with it. `crates/tests` and
benchmarks use the same builder with in-memory stores.

This crate holds no protocol rules. Put a protocol decision in the crate that owns it, and attach it
here.

## Map

| File                              | Owns                                                                                              |
| --------------------------------- | ------------------------------------------------------------------------------------------------- |
| `src/builder/mod.rs`              | `CiphernodeBuilder`, its options and defaults, and the two profiles `full` and `bootstrap`.       |
| `src/builder/startup.rs`          | `build`, the order of the phases, and their outputs `Storage`, `Restart`, `Live`, and `Protocol`. |
| `src/builder/evm.rs`              | The contract readers and writers of each enabled chain (`setup_evm_system`).                      |
| `src/builder/chain_parameters.rs` | The chain values that the extensions read: DKG fold contexts and the accusation vote validity.    |
| `src/builder/extensions.rs`       | The request router, the ZK actor recovery, and the protocol extensions of a full node.            |
| `src/builder/sortition.rs`        | Sortition and the ciphernode selector, restored from their snapshots.                             |
| `src/builder/net.rs`              | The network policy, libp2p or the in-process channel bridge, and the startup event buffer.        |
| `src/builder/restart.rs`          | Pure restart decisions from the local E3 lifecycle stages.                                        |
| `src/recovery.rs`                 | Restart-state backfill and committee snapshot reconciliation.                                     |
| `src/finalized_lifecycle.rs`      | Reconciliation of the restored contexts with the finalized chain lifecycle.                       |
| `src/ciphernode.rs`               | `CiphernodeHandle` and `NetInterfaceKind`.                                                        |

## Startup order

`build_inner` in `src/builder/startup.rs` sets the startup order. The module doc and the comment at
each step give the reasons. Each phase takes the output of the phase before it. Do not move a step
before a step that it needs.

## Rules for changes

- The node has two profiles: `NodeProfile::Full` and `NodeProfile::Bootstrap`. A bootstrap node
  reads only the Interfold contract, has no contract writer, and gets no protocol extension, so it
  signs no protocol message. Gate full-node work on `self.role() == NodeRole::Full`, or on the
  `NodeProfile::Full` match in `setup_extensions`. Do not add a setter that enables one component.
- To add an option, add a field to `CiphernodeBuilder`, set its default in `CiphernodeBuilder::new`,
  and add one `with_*` method. Check numeric limits in `ensure_startup_options`.
- A test-only option uses the `test-helpers` feature and the `_for_testing` suffix. The
  `test-only-skip-proof-aggregation` feature gates `with_proof_aggregation_disabled_for_testing`.
- Only a `ProviderCache<WriteEnabled>` can sign, and `go_live` creates it. The phases before
  `go_live` get a `ProviderCache<ReadOnly>`.
- Use `Chains` and `EnabledChain` from `e3-evm` for chain lookups. A disabled chain has no provider,
  so code that reads a chain uses `chains.enabled()`. `Protocol::slashing_managers` is in the order
  of `chains.iter()`.
- `e3-test-helpers` must not depend on this crate. The in-process multi-node harness
  `CiphernodeSystem` is in `crates/tests/tests/ciphernode_system/`. When the tests of a lower crate
  depend on this crate, they compile the full protocol stack, and they can make a dependency cycle
  with `e3-event-system`.

## Tests

- `cargo test -p e3-ciphernode-builder` runs the unit tests. The `finalized_lifecycle` tests start
  Anvil, so they need Foundry.
- The multi-node tests are in `crates/tests`. The full suite is slow: run only the tests that cover
  the change, with `--exact`.

## Look up

- The startup sequence and the bootstrap profile: `agent/CRATES_ARCHITECTURE.md` §Ciphernode
  construction and startup.
- The SlashingManager resolution: `agent/CRATES_ARCHITECTURE.md`, search `slashingManager()`.
- The restart and lifecycle invariants: `agent/invariants/03_ACTOR_RUNTIME.md`, search
  `finalized_lifecycle.rs` and `recovery.rs`.
- The extensions of each protocol step: `agent/flow-trace/03_E3_REQUEST_AND_COMMITTEE.md`, search
  `setup_extensions`.
