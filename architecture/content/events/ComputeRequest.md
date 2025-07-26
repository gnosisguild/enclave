---
type: event
description: Requests a computation
tags:
  - todo
  - trbfv
---

## `=this.file.name`

`=this.description`

Often payloads here are sensitive. Eg. `SecretKeyshare`, `BroadcastShares`

The event can have various request types which should be a nested enum representing different schemes we can use.

```rust
#[derive(Message)]
struct ComputeRequest {
	cmd: ComputeCommand,
	input: Sensitive<BroadcastShare>
	// ...
}

enum ComputeCommand {
  Trbfv(TrbfvCommand),
  Bfv(BfvCommand),
  Tfhe(TfheCommand)
  // ...
}
```


That way we can match on various schemes and commands.

### TrbfvCommand
#### GenerateBroadcastShares

Generate broadcast shares for sharing with other parties

#### DecryptCiphertext

Decrypt ciphertext

#### SumCollectedShares

Sum collected shares into the SecretKeyshare

#### GetAggregatePublicKey

Aggregate a public key from publickey shares

#### GetAggregatePlaintext

Aggregate a plaintext from decryption shares