# e3-test-helpers

Reusable low-level fixtures for capability tests: isolated event buses, seeded RNGs/BFV parameters,
sample shares, a mock network, application helpers, and tool discovery. Complete multi-node
composition lives in [e3-tests](../tests/README.md), not this dependency layer.

## Fixture APIs

- `get_common_setup(optional_param_set)` returns a bus, RNG, seed, BFV parameters/CRP, error
  collector, and history collector inside an Actix runtime.
- `create_shared_rng_from_u64(seed)` is deterministic; `derive_shared_rng(base_seed, salt)` gives
  each simulated node a separate mutex. These are test RNGs, not production entropy.
- `usecase_helpers` generates threshold shares, public keys, and decryption keys; `application`
  produces sample encrypted inputs/results.
- `libp2p_mock::Libp2pMock` connects channel bridges, including disconnect/replacement-node
  scenarios. `AddToCommittee` publishes synthetic registration events; neither helper establishes
  peer authorization or real chain finality.
- `find_bb()` and `find_anvil()` discover external tools; `with_tracing(level)` captures logs.

The [integration harness](../tests/tests/integration.rs) demonstrates these helpers alongside the
real builder. A mock transport or generated share is not itself a proof-verification test.

## Prerequisites before invoking Cargo

From the repository root, with JS dependencies installed:

```sh
pnpm evm:build
pnpm fixtures:build
cargo test --locked -p e3-test-helpers
```

Have bash, solc, and jq available. Foundry's Anvil is needed for chain paths. The transitive
[prover build script](../zk-prover/README.md) can invoke pnpm/Noir builds and rewrite fixtures even
during a Cargo check; read its compile-only flag and real-proof prerequisites first. Use the pinned
[Rust toolchain](../../rust-toolchain.toml) and matching circuit/Barretenberg tools.

Tool discovery lets some consumers skip when bb/Anvil is missing. Check test output for skips: a
successful helper setup or test command does not prove a real chain or crypto path executed.
