# e3-utils

Shared mechanical helpers for runtime/adapters: bounded decoding, byte wrappers, retry and receipt
handling, Actix/Tokio channels, paths, and diagnostic formatting. Capability-specific protocol
decisions live in their owning crates.

## Decode bounded data

```rust
use e3_utils::deserialize_bounded;

let encoded = bincode::serialize(&vec![1_u64, 2, 3]).unwrap();
let decoded: Vec<u64> = deserialize_bounded(&encoded, 1024).unwrap();
assert_eq!(decoded, vec![1, 2, 3]);
```

`deserialize_bounded(bytes, max_bytes)` uses the repository's legacy bincode-1 fixed-integer format,
enforces the byte budget through a reader, and rejects trailing bytes. `deserialize_exact` uses the
supplied encoded length as its budget. Neither validates application authorization or the semantic
meaning of the decoded value.

## Other entry points

- `ArcBytes` shares byte vectors; `SharedRng` is the common mutex-backed ChaCha20 RNG type.
- `AsBytesSerde` and the re-exported `BytesSerde` derive encode human-readable hex versus binary
  byte strings. See [derive usage](../utils-derive/README.md) for caller dependencies/validation.
- `retry_with_backoff` distinguishes `RetryError::Retry` and `RetryError::Failure` according to
  caller classification. `require_successful_receipt` checks a transaction receipt rather than
  mistaking RPC submission for successful execution.
- `actix` exposes message/channel adapters and `NotifySync`; `OnceTake` transfers a shared value at
  most once. Awaited acknowledgments and fire-and-forget sends have different contracts.
- `enumerate_path`, ordering, and formatting helpers avoid duplicating mechanical conversions.

The [public module exports](src/lib.rs) are the navigation point for specialized helpers. Run
`cargo test --locked -p e3-utils` for decoding and macro-consumer coverage. Serialized helpers serve
saved/wire data; see [upgrades](../../docs/architecture/upgrades.md) for compatibility.
