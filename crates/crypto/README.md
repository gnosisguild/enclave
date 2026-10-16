# e3-crypto

Local encryption for saved operator identities and secret shares. It provides password sources, an
Argon2-derived AES-GCM `Cipher`, and encrypted byte wrappers. Threshold BFV calculations belong in
[e3-trbfv](../trbfv/README.md), not this crate.

## Protect local bytes

```rust
use e3_crypto::{Cipher, SensitiveBytes};

async fn round_trip(password: &str) -> anyhow::Result<()> {
    let cipher = Cipher::from_password(password).await?;
    let secret = SensitiveBytes::new(vec![1, 2, 3], &cipher)?;
    let clear = secret.access(&cipher)?;
    assert_eq!(clear.as_slice(), &[1, 2, 3]);
    Ok(())
}
```

In production, obtain the password through `Cipher::from_file` or `Cipher::from_env` rather than
hard-coding it. `Cipher::new` accepts a custom `PasswordManager`; built-ins support in-memory,
environment, and file sources. File password creation refuses overwrite and uses restrictive Unix
permissions.

`SensitiveBytes` serializes encrypted bytes. `access()` returns a zeroizing plaintext buffer;
`access_raw()` instead makes the caller responsible for cleanup. Low-level `encrypt_data` zeroizes
its successful input and returns nonce-prefixed authenticated ciphertext. The current password
derivation uses a fixed application salt, not a per-record random salt. Keep passwords and decrypted
buffers out of logs and ordinary snapshots.

Run `cargo test --locked -p e3-crypto`; file tests use Unix filesystem behavior. Saved ciphertext
and KDF choices are durable compatibility boundaries; see
[upgrades](../../docs/architecture/upgrades.md) before changing stored data.
