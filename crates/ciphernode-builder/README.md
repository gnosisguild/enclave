# e3-ciphernode-builder

Library composition root for a ciphernode: it connects storage, chain adapters, networking, request
routing, and protocol capabilities. It is not an executable; normal operators use the
[CLI](../cli/README.md), whose [entrypoint](../entrypoint/README.md) also owns process fencing.

## Choose a node profile

The public constructors are:

```rust
// Signatures; types come from e3-utils, e3-crypto, and e3-zk-prover.
pub fn full(rng: SharedRng, cipher: Arc<Cipher>, zk_backend: ZkBackend) -> Self;
pub fn bootstrap(rng: SharedRng, cipher: Arc<Cipher>) -> Self;
```

- **Full** installs threshold keyshare, public-key/plaintext aggregation, proof workers, accusation
  voting, commitment checks, and protocol contract readers/writers.
- **Bootstrap** serves discovery/history with networking and the Interfold reader. It does not join
  committees, prove, sign protocol messages, or send protocol transactions. It still needs an
  operator identity and uses routing, lifecycle, sortition, and DA infrastructure. It may start
  without peer history when none is available.

Persisted directories are role-stamped: full and bootstrap nodes cannot reuse each other's store.

## Configure and start

The production recipe is [entrypoint startup](../entrypoint/src/start/start.rs): seed a `SharedRng`,
load the password-backed `Cipher`, choose a profile, then configure:

- `with_chains(&[ChainConfig])` and `with_network(profile, peers, quic_port)`;
- `with_persistence(&log_path, &kv_path)` (otherwise storage is in memory);
- `with_multithread_config(reserve_threads, concurrent_jobs)` for full-node CPU/memory admission;
- startup buffer limits, name/logging, and optional shared stores for local commands.

`build().await -> anyhow::Result<CiphernodeHandle>` completes initial replay and live-source
readiness before returning. Run it inside an Actix runtime; its actor setup uses local tasks, so do
not move the startup future to an arbitrary `tokio::spawn` worker. Enabled chains need reachable RPC
endpoints and deployed contracts. Full nodes also need matching
[Noir/Barretenberg artifacts](../zk-prover/README.md); preparation may install artifacts. The
entrypoint applies a configured startup deadline around `build()`.

## Handle and shutdown

`CiphernodeHandle` exposes `bus()`, `eventstore()`, `aggregate_ids()`, `network_status()`, and the
operator address/peer ID. `with_history_collector()` enables the optional `history()`. Watch
`persistence_health()` and `gateway_failure()` for fatal storage or chain-ingestion failure; these
are explicit signals, not supervision of every actor panic.

Consume the handle with `shutdown(deadline).await`: it acknowledges shutdown handlers, flushes the
event pipeline, then drains snapshots and closes storage within the supplied deadline. See
[persistence and replay](../../docs/architecture/persistence-and-replay.md) for guarantees and
recovery limits, and [e3-tests](../tests/README.md) for in-process multi-node usage.
