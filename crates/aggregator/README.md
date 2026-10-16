# e3-aggregator

Ciphernode capabilities for assembling a committee public key and decrypting the computed ciphertext
from threshold shares. These capabilities retain phase/recovery state; proof workers and EVM writers
perform the expensive computation and on-chain publication.

## Runtime integration

Use the extensions in `e3_aggregator::ext` with an `e3_request::E3RouterBuilder`:

- `AggregatorRoleExtension::create(initial_roles)` supplies restored per-E3 aggregator roles.
- `PublicKeyAggregatorExtension::create(bus)` creates or hydrates the `publickey` recipient.
- `ThresholdPlaintextAggregatorExtension::create(...)` creates or hydrates `plaintext`, using the
  bus, sortition actor, proof-aggregation setting, canonical keys, and sequence-history reader.

The [ciphernode wiring](../ciphernode-builder/src/builder/extensions.rs) shows registration through
`with_recipient`, including the FHE dependency. Consumers do not construct the private
public-key/plaintext actor modules directly. `CommitteeFinalizer` is a separate node-wide actor; its
[startup caller](../ciphernode-builder/src/builder/startup.rs) provides recovery storage and
fresh-provider factories for the enabled chains.

Aggregation follows the canonical roster and active/standby roles, not the fastest arriving shares.
Plaintext reconstruction needs T+1 valid roster shares. Local aggregation results are publication
intents; confirmed contract events establish canonical completion.

## Using and testing

For a complete runtime use [CiphernodeBuilder](../ciphernode-builder/README.md). The crate also
exports typed recovery states and repository factory traits for persistence integration. See
[DKG](../../docs/architecture/dkg.md) and
[threshold decryption](../../docs/architecture/threshold-decryption.md) for protocol boundaries.

Focused tests: `cargo test --locked -p e3-aggregator` from the repository root. Prepare the
[test-helper prerequisites](../test-helpers/README.md) first; the test graph includes chain fixtures
and the prover. Actor tests with mock proofs do not validate recursive proofs.
