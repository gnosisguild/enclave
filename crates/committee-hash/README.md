# e3-committee-hash

Deterministic EVM committee and decryption-domain hashes shared by Rust protocol consumers. There
are no provider lookups: callers supply the canonical addresses and E3 chain context.

## Committee commitments

```rust
use alloy::primitives::Address;
use e3_committee_hash::{hash_committee_addresses, split_committee_hash};

let ordered = [Address::from([1_u8; 20]), Address::from([2_u8; 20])];
let hash = hash_committee_addresses(&ordered);
let limbs = split_committee_hash(hash);
assert_eq!(&limbs.hi.as_slice()[16..], &hash.as_slice()[..16]);
```

Hashing is Keccak-256 over ordered raw 20-byte addresses, with no sorting, deduplication, or
per-address ABI padding. `committee_hash_limbs_from_addresses` combines hashing/splitting;
`committee_hash_field_hex` returns the two right-aligned field hex strings for Noir inputs.

## Decryption domains

`hash_decryption_domain(chain_id, e3_id, context, ciphertext_output_hash)` hashes the Solidity ABI
tuple that binds chain, Interfold deployment, E3, committee hash, output ciphertext, and committee
public-key commitment. Supply the latter three context fields through `DecryptionDomainContext`.
`split_decryption_domain` and `decryption_domain_limbs` expose the two u128 Noir limbs. This domain
is not interchangeable with the committee hash alone.

Run `cargo test --locked -p e3-committee-hash` for cross-language vectors. The larger C6/C7 contract
is documented in [threshold decryption](../../docs/architecture/threshold-decryption.md).
