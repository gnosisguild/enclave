# e3-safe

SAFE sponge over BN254 field elements with Poseidon2, shared by native commitment code and the
OpenVM guest. It binds a prescribed absorb/squeeze transcript and a 64-byte domain separator; it is
not a generic byte-hashing convenience API.

## Sponge transcript

```rust
use e3_safe::{Field, SafeSponge, ABSORB_FLAG, SQUEEZE_FLAG};

// Example domain only; protocol callers use their prescribed domain separator.
let pattern = [ABSORB_FLAG | 2, SQUEEZE_FLAG | 1];
let mut sponge = SafeSponge::start(pattern, [0_u8; 64]);
sponge.absorb(vec![Field::from(1_u64), Field::from(2_u64)]);
let digest = sponge.squeeze();
sponge.finish();
assert_eq!(digest.len(), 1);
```

The low 31 bits encode each operation's element count; the high bit selects absorb. `squeeze()`
obtains its output count from the pattern. Mismatched operation order/lengths, an exhausted pattern,
or incomplete `finish()` panic. `compute_tag` binds the pattern and domain. The sponge has rate
three and capacity one. Field conversion and domain choice belong to each commitment statement, not
to implicit normalization here.

## Native and guest builds

Default builds use the native Poseidon2 implementation. The `openvm` feature enables guest
integration; `phase-trace` adds guest diagnostic tracing. Guest cfg/feature combinations are
target-specific, so enabling every feature on a host is not a guest parity check.

Run `cargo test --locked -p e3-safe` for native/reference vectors. For OpenVM execution follow the
[worker workflow](../openvm-prover/README.md). Cross-language commitment requirements live in
[proof statements](../../docs/architecture/proof-statements.md).
