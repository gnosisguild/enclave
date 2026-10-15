# e3-event-system

Read this file before you change this crate. `agent/RULES.md` still applies.

## Purpose

This crate connects the event runtime of one node or one test: the event bus, one event store for
each aggregate, the event store router, the sequencer, the snapshot buffer, and the data store.
`EventSystem` creates each part on first use, and then keeps it. It has two backends:

- In memory: `EventSystem::in_mem()`, which `EventSystem::new()` also returns.
- Persisted: `EventSystem::persisted(log_path, sled_path)`, with commitlog event logs and a sled
  store.

## Map

| File                             | Owns                                                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `src/event_system.rs`            | `EventSystem`, the two backends, `EventStoreAddrs`, and the tests.                                           |
| `src/eventbus_factory.rs`        | The process-wide default bus: `get_interfold_event_bus` and `get_interfold_bus_handle`.                      |
| `src/global_store_cache.rs`      | `share_store` and `get_cached_store`: the store that socket commands use while a node runs.                  |
| `src/global_eventstore_cache.rs` | `EventStoreReader` (sequence and timestamp queries), `share_eventstore_reader`, and `get_shared_eventstore`. |

## Rules for changes

- Call `with_event_bus`, `with_fresh_bus`, and `with_aggregate_config` before the first getter. Each
  part is kept in a `OnceCell`, so a setter after the first getter has no effect.
- In a test, use `with_fresh_bus()`. Without `with_fresh_bus` or `with_event_bus`, the system uses
  the process-wide default bus, and the tests share events.
- The persisted backend checks the schema with `e3_sync::inspect_persisted_schema_version` before it
  opens a log or a sequence index. Keep this order.
- The persisted backend makes one event store for each ID in `aggregate_config().indexed_ids()`. The
  log path of an aggregate is `enumerate_path(log_path, index)`, and its sled tree is
  `sequence_index.{index}`. A change to these names moves durable data, so it is a protocol-bearing
  change (`agent/flow-trace/07_UPGRADES.md`).
- `enable_handle_with_hlc` reads `EventStoreClockFloor` from each event store, and seeds the bus
  clock above the highest durable timestamp before it returns the enabled bus. A new path that
  enables the bus must do the same.
- `handle()` subscribes the snapshot buffer with `SubscribePreFanout` before it creates the
  `BusHandle`.
- To send a message to both store kinds, use `EventStoreAddrs::recipients::<M>()` or
  `RouterAddr::recipient::<M>()`. Both store types must implement `Handler<M>`.
- The global caches keep only the first value. Only a production node enables sharing, through
  `CiphernodeBuilder::with_shared_stores`.
- This crate depends on `e3-data`, `e3-events`, `e3-sync`, and `e3-utils`. Do not add a dependency
  on a protocol crate or on `e3-ciphernode-builder`: many crates use this crate in their tests.

## Tests

`cargo test -p e3-event-system`

## Look up

- Persistence, replay, and the schema check: `agent/CRATES_ARCHITECTURE.md` §Event ingestion,
  persistence, replay, and synchronization.
- The log layout and upgrades: `agent/flow-trace/07_UPGRADES.md`, search `EventSystem`.
- The runtime invariants: `agent/invariants/03_ACTOR_RUNTIME.md`.
