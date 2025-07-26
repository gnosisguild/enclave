---
type: event
description: Requests a computation
tags:
  - todo
  - trbfv
  - compute
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
#### TrBfv::GenerateBroadcastShares

Generate broadcast shares for sharing with other parties

#### Trbfv::DecryptCiphertext

Decrypt ciphertext

#### Trbfv::SumCollectedShares

Sum collected shares into the SecretKeyshare

#### Trbfv::GetAggregatePublicKey

Aggregate a public key from publickey shares

#### Trbfv::GetAggregatePlaintext

Aggregate a plaintext from decryption shares