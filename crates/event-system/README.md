# e3-event-system

Runtime assembly of the event bus, per-aggregate event stores, sequencer, snapshot buffer, and data
store. [e3-events](../events/README.md) owns their messages/types and pipeline behavior;
[e3-data](../data/README.md) provides storage implementations. This crate wires them without
requiring a full ciphernode.

## Create a runtime

Choose `EventSystem::in_mem()` (`new()`/Default are equivalent) or
`EventSystem::persisted(log_path, sled_path)`. Both expose the same handles and query recipients.
Persisted setup checks schema compatibility before opening aggregate logs/indexes.

Configure `with_aggregate_config(...)` and either `with_fresh_bus()` or `with_event_bus(...)` before
calling getters. Initialization is lazy and keeps the first value; the uncustomized bus is
process-wide. Independent tests/nodes should use a fresh or explicit bus.

Inside an Actix runtime, a minimal isolated setup is:

```rust
use e3_event_system::EventSystem;
use e3_events::hlc::Hlc;

async fn setup() -> anyhow::Result<()> {
    let system = EventSystem::in_mem().with_fresh_bus();
    let store = system.store()?;
    let bus = system.enable_handle_with_hlc(Hlc::from_str("example")).await?;
    bus.flush_event_pipeline().await?;
    store.shutdown().await?;
    Ok(())
}
```

## Handles and health

`handle()` returns a disabled bus and registers snapshot pre-fanout admission.
`enable_handle_with_hlc` additionally waits for registration and seeds the clock above every store's
durable maximum before publication. Use that path for restored production stores, rather than
directly enabling a handle with an unseeded clock.

`store()` connects the snapshot buffer to the data store; `eventstore_reader()` provides
sequence/timestamp query recipients. Lower-level `buffer()`, `sequencer()`, and `eventstore_addrs()`
support specialized assembly. Watch `failure_receiver()` for explicit persistence/admission failure
signals. Global shared-store/reader caches are opt-in and process-local, not inter-process database
sharing.

For production lifecycle/replay use [CiphernodeBuilder](../ciphernode-builder/README.md) and the
[persistence contract](../../docs/architecture/persistence-and-replay.md). Tests:
`cargo test --locked -p e3-event-system`, with [helper prerequisites](../test-helpers/README.md).
