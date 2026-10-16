# e3-evm

## Purpose

Ciphernode EVM adapters: canonical log ingestion, contract readers/writers, chain/provider
resolution, and confirmed-key/lifecycle projections. Application-side listeners and contract clients
live separately in [e3-evm-helpers](../evm-helpers/README.md).

## Map

| Location                                                                                                                      | Responsibility                                                                     |
| ----------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [`chains.rs`](src/chains.rs), [`provider_cache.rs`](src/provider_cache.rs), [`chain_system.rs`](src/chain_system.rs)          | Chain/provider resolution and per-chain route composition.                         |
| [`chain_reader/`](src/chain_reader), [`log_fetching/`](src/log_fetching)                                                      | Confirmed backfill/live reads, adaptive windows, timestamps, and progress.         |
| [`event_decoding/`](src/event_decoding), [`historical_order/`](src/historical_order), [`chain_gateway/`](src/chain_gateway)   | Decode observations, hand off historical/live input, and report readiness/failure. |
| [`interfold/`](src/interfold), [`ciphernode_registry/`](src/ciphernode_registry), [`bonding_registry/`](src/bonding_registry) | Contract-specific reader projections.                                              |
| [`interfold_writing/`](src/interfold_writing), [`slashing_writing/`](src/slashing_writing)                                    | Publication preflights/retries and slash outbox.                                   |
| [`canonical_key.rs`](src/canonical_key.rs), [`data_availability/`](src/data_availability)                                     | Confirmed key authority and referenced-output retrieval.                           |

The read path fetches confirmed logs, decodes observations, orders the startup handoff, and sends
facts to the bus. The write path consumes local intents; confirmed results return through the read
path rather than treating transaction submission as completion.

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

## Tests and build prerequisites

From the root, prepare `pnpm evm:build` and `pnpm fixtures:build` with workspace JS dependencies,
bash, solc, and jq available. This crate's `build.rs` invokes fixture compilation even for
`cargo check` and can rewrite its emit-logs fixture JSON. Tests that use a local chain need
Foundry's Anvil on PATH. Run `cargo test --locked -p e3-evm`; local tests are not proof of reorg
rollback or exactly-once transaction submission.

Unit cases are beside decoding, fetch windows, gateways, and publication workflows.
[`tests/integration.rs`](tests/integration.rs) uses local logs for history/owner/admission recovery;
[`live_sepolia_window.rs`](tests/live_sepolia_window.rs) is a separately selected live-endpoint
diagnostic with its own environment requirements. [AGENTS.md](AGENTS.md) covers reader/writer
changes.
