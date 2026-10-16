# e3-request

Per-E3 context routing and lifecycle projection. The router admits requests from chain facts or
restored contexts, constructs their capability recipients through extensions, and forwards events.
It does not own the cryptographic statements implemented by those recipients.

## Build a router

`E3Router::builder(bus, store)` supplies the metadata extension by default. Add capabilities with
`E3RouterBuilder::with(extension)` or `with_recipient("recipient_name", extension)`, then await
`build()` to hydrate and attach. Named recipients let the router defer relevant history until the
extension creates its actor. Use the
[ciphernode extension wiring](../ciphernode-builder/src/builder/extensions.rs) for a complete
composition, including restored selections and terminal-state gates.

`E3Extension` implements synchronous `on_event(context, event)` and async
`hydrate(context, snapshot)`. An extension can inject typed dependencies with
`E3Context::set_dependency` or register an Actix recipient with `set_event_recipient`. `TypedKey<T>`
retrieves dependencies without capability-specific casts. `E3MetaExtension` populates `META_KEY`;
`DKG_FOLD_ATTESTATION_CONTEXT_KEY` carries the corresponding proof context.

## Lifecycle and key authority

`E3LifecycleCoordinator` is a separate node-wide observer with persisted stage projection; it is
attached by [builder startup](../ciphernode-builder/src/builder/startup.rs). Terminal local state
does not itself prove confirmed chain completion, and failed contexts can remain alive for
accusation work. Peer traffic cannot create an admitted E3.

`canonical_key::CanonicalPublicKeys` is the shared cache for confirmed key identity and
decryption-domain repair. Its EVM projection establishes authority; a local `PublicKeyAggregated`
intent cannot. Repository factory traits expose typed context, metadata, lifecycle, and
router-checkpoint storage to startup/recovery consumers.

See [persistence and replay](../../docs/architecture/persistence-and-replay.md) for checkpoint,
deferred-queue, overflow, and teardown limits. Run `cargo test --locked -p e3-request`; routing
capacity tests and full node restart scenarios cover different boundaries.
