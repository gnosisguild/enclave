# e3-layout-lock

Test utility that samples serde layouts and locks their bincode encodings in reviewed fixtures.
Consumers list the persisted/wire roots they want covered; this is not a schema migration tool.

## Lock a consumer type

```rust
use e3_layout_lock::{assert_fixture, sample_rows};
use std::path::Path;

let rows = sample_rows::<Option<u64>, _>("optional-counter", |_| String::new());
assert_fixture(Path::new("tests/fixtures/optional-counter.txt"), &rows);
```

The fixture must already contain the accepted rows for normal test runs. `sample_rows` requires
`Serialize + DeserializeOwned`; its callback adds consumer-specific information to each row. See the
[runtime layout test](../tests/tests/layout_lock.rs) for multiple roots, repository value bindings,
and event-ID samples.

Rows contain a named digest (sample/layout names and structure) and positional digests (encoded
sample bytes independent of cosmetic names). Enum paths, optional/empty/full shapes, and flipped
booleans expose many changes that a single round trip would miss.

## Updating and limits

`LAYOUT_LOCK_UPDATE=1` rewrites a consumer fixture instead of comparing it. Only use it after
reviewing the intended format change and the
[schema/wire compatibility decision](../../docs/architecture/upgrades.md). For the broad runtime
fixture the command is `LAYOUT_LOCK_UPDATE=1 cargo test --locked -p e3-tests --test layout_lock`;
that crate has [additional prerequisites](../tests/README.md).

Unlisted roots, hand-written framing, and values inside opaque/encrypted byte strings are not
covered. Some equal-width signedness changes and repeated generic enum names also escape the
samples; the [crate-level documentation](src/lib.rs) describes these limits. A matching lock is not
proof of semantic compatibility. Focused utility tests: `cargo test --locked -p e3-layout-lock`.
