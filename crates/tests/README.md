# e3-tests

## Purpose

Cross-capability and multi-node ciphernode test harness. `src/main.rs` intentionally does no work;
use Cargo's named integration targets, not `cargo run`, to exercise the system.

## Map

| Location                                                                 | Responsibility                                                           |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| [`tests/ciphernode_system/`](tests/ciphernode_system)                    | Multi-node builder/handle harness, histories, and network helpers.       |
| [`tests/integration.rs`](tests/integration.rs)                           | Cross-capability scenarios and optional expensive proof/benchmark flows. |
| [`tests/large_work_plan_recovery.rs`](tests/large_work_plan_recovery.rs) | Reopen large proof plans and dispatch missing work.                      |
| [`tests/layout_lock.rs`](tests/layout_lock.rs)                           | Runtime encoding roots and repository/event samples.                     |
| [`src/main.rs`](src/main.rs)                                             | Empty executable, not a test runner.                                     |

Scenarios compose real actors with isolated buses/stores and selectable transport/worker
substitutes. They publish histories, observe outcomes, and for recovery reopen saved state. Which
parts are mocked or ignored is specific to the scenario, not guaranteed by the package name.

## Tests: choose a target

- [integration.rs](tests/integration.rs): end-to-end key/plaintext flows, routing, restart, and
  multi-node scenarios using [ciphernode_system](tests/ciphernode_system/mod.rs).
- [large_work_plan_recovery.rs](tests/large_work_plan_recovery.rs): persisted large-work-plan
  recovery scenarios.
- [layout_lock.rs](tests/layout_lock.rs): listed persisted/wire serde roots, repository value types,
  and event-ID samples. It does not cover store paths, manual framing, or opaque encrypted payloads;
  see [layout-lock](../layout-lock/README.md).

After preparing tools, run a focused target from the repository root:

```sh
cargo test --locked -p e3-tests --test layout_lock
cargo test --locked -p e3-tests --test integration -- --list
```

Select a named scenario from the list rather than launching every expensive integration or ignored
proof benchmark. [Root package scripts](../../package.json) provide the broader integration and
real-proof suites.

### Synchronized-decryption failure scenarios

Prepare a complete matching artifact pair first. `test_trbfv_actor` keeps recursive proof
aggregation enabled by default. `BENCHMARK_RESTART_PARTY_ID=<zero-based ID>` together with
`BENCHMARK_RESTART_DURING_DECRYPTION=1` restarts a selected non-aggregator from disk after its C6
intent is recorded, before share publication.

With complete `insecure-64/small` artifacts active, run:

```sh
BENCHMARK_REPLACE_DECRYPTOR=1 cargo test --locked -p e3-tests --test integration test_trbfv_actor -- --nocapture --test-threads=1
```

This requires one compute slot (the default), recursive proofs and no restart injection. It holds
compute admission while delivering a quorum-confirmed exclusion through every node's durable
pipeline, observes converged replacement requests, then releases a retired share calculation. It
checks that the stale response starts no C6 proof, all published shares use the replacement set, and
the normal real-proof plaintext flow completes. This models exclusion delivery, not accusation
consensus or on-chain slashing. Small has an H=14 roster for T+1=10 decryptors; Minimum and Micro
have no spare H-roster member and deliberately cannot run this replacement scenario.

## Prerequisites and assurance

Prepare root `pnpm evm:build` and `pnpm fixtures:build`, with bash, solc, jq, and Anvil for chain
scenarios. [Prover setup](../zk/prover/README.md) explains matching nargo/bb/artifacts and
build-time fixture side effects. Even the layout target compiles the runtime dependency graph. Tests
may skip when tools are absent, and ignored scenarios need explicit selection.

This harness enables builder `test-helpers` and `test-only-skip-proof-aggregation`. Cargo feature
unification can enable them throughout a workspace test graph; successful tests do not establish
production feature containment or recursive proof verification. Deterministic RNGs and mocked
transport are isolated per node but do not model all distributed failures. See
[persistence and replay](../../docs/architecture/persistence-and-replay.md) for current
restart/overflow assurance limits. See [AGENTS.md](AGENTS.md) for isolation and regression-design
guidance.
