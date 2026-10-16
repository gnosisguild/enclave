# e3-bfv-client

Native client helpers for BFV encryption, circuit-compatible public-key/ciphertext commitments, and
plaintext byte conversion. The same implementation backs the [WASM API](../wasm/README.md) and is
re-exported as `e3_sdk::bfv_client` with the SDK's `bfv` feature.

## Encrypting inputs

`client::bfv_encrypt(data, public_key, degree, plaintext_modulus, moduli)` takes serialized
public-key bytes and BFV parameters and returns serialized ciphertext bytes. Parameters must match
the committee key; use [e3-fhe-params](../fhe-params/README.md) rather than copying presets.

For inputs that need an encryption proof, `client::bfv_verifiable_encrypt(...)` returns
`VerifiableEncryptionResult { encrypted_data, circuit_inputs }`. The JSON circuit inputs contain the
encryption witness/randomness for that same ciphertext. This produces witness data, not a proof, and
only supported threshold presets are accepted. Keep that JSON private.

Before trusting a distributed key, call `client::validate_pk_commitment` with its expected on-chain
commitment and parameters. Validation checks the decoded key's circuit commitment, not byte-for-byte
decode/re-encode equality. `compute_pk_commitment`, `compute_ct_commitment`, and
`compute_ct_commitment_with_params` expose the corresponding commitment calculations.
`generate_public_key` is useful for fixtures: it discards the secret key, so it is not a recoverable
client key-generation API or a distributed committee setup.

## Plaintext words

```rust
use e3_bfv_client::{decode_bytes_to_vec_u64, encode_vec_u64_to_bytes};

let words = vec![1_u64, 2, 3];
let bytes = encode_vec_u64_to_bytes(&words);
assert_eq!(decode_bytes_to_vec_u64(&bytes).unwrap(), words);
```

Words are little-endian; decoding rejects a partial eight-byte word.

Run `cargo test --locked -p e3-bfv-client` from the repository root. Encryption round trips do not
replace the [proof-statement](../../docs/architecture/proof-statements.md) acceptance flow.
