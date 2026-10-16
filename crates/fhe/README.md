# e3-fhe

Runtime adapter around fhe.rs for keyshare generation, ciphertext decryption, and public-key/
plaintext share aggregation. It also supplies an FHE dependency to each admitted E3 context; DKG
orchestration itself belongs in [keyshare](../keyshare/README.md).

## Direct adapter use

`Fhe::new(params, crp, rng)` accepts BFV parameters, a common random polynomial, and a shared RNG.
`from_encoded(bytes, rng)` decodes request parameters; `from_raw_params` accepts explicit moduli,
degree, plaintext modulus, and CRP bytes plus the RNG.

Methods take typed inputs rather than owning protocol routing:

- `generate_keyshare()` returns serialized secret/public-share material.
- `decrypt_ciphertext(DecryptCiphertext)` produces a serialized decryption share.
- `get_aggregate_public_key(GetAggregatePublicKey)` combines serialized key shares.
- `get_aggregate_plaintext(GetAggregatePlaintext)` combines decryption shares and returns the client
  crate's little-endian plaintext-word bytes.

See [runtime.rs](src/runtime.rs) for input fields. Secret-key serialization contains raw
coefficients and must remain behind encrypted-storage/caller boundaries.

## Per-request integration

Register `ext::FheExtension::create(bus, rng)` on `E3RouterBuilder`. On `E3Requested` it stores the
request's FHE snapshot and injects `Arc<Fhe>` at `ext::FHE_KEY`; it does not register an
event-recipient actor. Hydration restores parameters/CRP and injects the supplied RNG separately.
The [builder caller](../ciphernode-builder/src/builder/extensions.rs) installs it alongside
aggregation extensions. `FheRepositoryFactory` supplies typed snapshot storage.

Run `cargo test --locked -p e3-fhe` from the repository root. See
[DKG](../../docs/architecture/dkg.md) and
[threshold decryption](../../docs/architecture/threshold-decryption.md) for the surrounding flow.
