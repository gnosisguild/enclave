# e3-evm

Ciphernode EVM adapters: canonical log ingestion, contract readers/writers, chain/provider
resolution, and confirmed-key/lifecycle projections. Application-side listeners and contract clients
live separately in [e3-evm-helpers](../evm-helpers/README.md).

## Runtime integration

`Chains::resolve(configs, &mut provider_cache).await` resolves enabled chains and validates their
configured IDs. `ProviderCache<ReadOnly>` serves startup inspection; explicit `with_write_support`
produces `ProviderCache<WriteEnabled>` for signer-backed effects.

`EvmSystemChainBuilder::new(bus, provider)` attaches contract-address routes and builds an
`EvmChainGatewayHandle`. Configure log windows, startup buffer limits, provider factories, and
optional ingestion progress before building. The composition root's
[EVM wiring](../ciphernode-builder/src/builder/evm.rs) is the complete reader/writer recipe;
operators should normally use [CiphernodeBuilder](../ciphernode-builder/README.md).

Subscriptions wake canonical backfill; they do not themselves prove complete ingestion. Configured
confirmation depth applies independently of whether the RPC URL is loopback. Gateway
readiness/failure handles expose live transition and fail-closed ingestion errors. Confirmed chain
history, not gossip or local aggregation intents, authorizes canonical keys and completion. Writer
preflights and retry results are separate from accepted chain facts.

Public helpers also include `ProviderConfig`, canonical DKG timing/lifecycle reads,
`fetch_operator_status`, and slashing-manager resolution. See
[committee selection](../../docs/architecture/committee-selection.md),
[threshold decryption](../../docs/architecture/threshold-decryption.md), and
[slashing](../../docs/architecture/slashing.md) for their cross-layer contracts.

## Build and test prerequisites

From the root, prepare `pnpm evm:build` and `pnpm fixtures:build` with workspace JS dependencies,
bash, solc, and jq available. This crate's `build.rs` invokes fixture compilation even for
`cargo check` and can rewrite its emit-logs fixture JSON. Tests that use a local chain need
Foundry's Anvil on PATH. Run `cargo test --locked -p e3-evm`; local tests are not proof of reorg
rollback or exactly-once transaction submission.
