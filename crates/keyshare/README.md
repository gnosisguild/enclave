# e3-keyshare

Per-E3 DKG and secret-share lifecycle for a ciphernode: collect authenticated dealer material,
produce threshold contributions, assemble this node's decryption key, and later emit proven
decryption shares. Arithmetic runs through [e3-trbfv](../trbfv/README.md) and shared workers; this
capability coordinates protocol state and recovery.

## Runtime integration

Register `ext::ThresholdKeyshareExtension::create(...)` on `E3RouterBuilder` with recipient name
`threshold_keyshare`. It creates `ThresholdKeyshare` on `CiphernodeSelected` and hydrates the
corresponding saved actor during restart. It is not one globally shared keyshare actor.

The constructor takes the bus, local encryption `Cipher`, operator address, per-chain Interfold
addresses, `DkgTimingReader`, signer, and shared `canonical_key::CanonicalPublicKeys`. The timing
reader returns the request's canonical deadline/window; metadata comes from the router context. See
the [production wiring](../ciphernode-builder/src/builder/extensions.rs) for dependencies and
named-recipient registration rather than reproducing a runtime fixture.

The crate exports actor construction parameters, recovery payload/state types, and
`ThresholdKeyshareRepositoryFactory` for specialized harnesses. `EncryptionKeyCollector` is the
dealer-key collection boundary. Canonical-key types are shared from `e3-request`; a local aggregate
intent is not permission to decrypt under an unconfirmed key.

Successful randomized contributions are reused after restart. Secret material is wrapped in local
encrypted storage; do not treat the exported state types as safe plaintext diagnostics. Detailed
dimensions, roster transitions, proof batches, and recovery limits are in
[DKG](../../docs/architecture/dkg.md) and
[threshold decryption](../../docs/architecture/threshold-decryption.md).

## Tests

Run `cargo test --locked -p e3-keyshare` after the
[test-helper prerequisites](../test-helpers/README.md). Its test graph includes EVM/prover build
steps. Actor/mock coverage is separate from matching real-proof acceptance.
