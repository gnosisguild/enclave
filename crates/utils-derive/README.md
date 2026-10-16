# e3-utils-derive

Procedural implementation of the `BytesSerde` derive, normally imported through `e3-utils`. It
generates serde implementations for a single-field tuple/newtype whose byte validation is provided
by `AsBytesSerde`.

## Define a byte newtype

```rust
use e3_utils::{AsBytesSerde, BytesSerde};

#[derive(BytesSerde)]
struct Digest([u8; 32]);

impl AsBytesSerde for Digest {
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
        let array = bytes.try_into().map_err(|_| "expected 32 bytes".to_string())?;
        Ok(Self(array))
    }
}
```

The consuming crate needs `e3-utils`, `serde`, and `hex` dependencies: generated paths resolve
there, not inside the macro crate. Human-readable formats serialize a `0x`-prefixed hex string;
deserialization accepts hex with or without that prefix. Binary serializers receive a byte string
rather than a text string. `try_from_bytes` must reject invalid lengths/content.

Named-field structs, multiple fields, and enums are rejected. The derive does not encrypt, redact,
or authorize bytes, and it does not validate semantics without the trait implementation. `ArcBytes`
in [e3-utils](../utils/src/utility_types.rs) is an existing consumer.

Run `cargo check --locked -p e3-utils-derive`, then `cargo test --locked -p e3-utils` to exercise
actual expansion. Macro compilation alone does not check generated consumer code.
