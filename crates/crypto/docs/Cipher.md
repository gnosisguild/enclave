---
type: lib
description: The cipher is a library that encrypts data to a secret key
source: https://github.com/gnosisguild/enclave/blob/main/crates/crypto/src/cipher.rs
tags:
  - todo
  - trbfv
  - crypto
---

## `=this.file.name`

`=this.description`

#### Description

This is used by the [[Keyshare]] actor to store it's single secret share. It is also used by the [[Sensitive]] container.

The cipher is a library that encrypts data to the key found in the keyfile the location of which is configured within the `enclave.config.yaml` 

Additions we should consider with the cipher:
- #todo Currently the key remains in the cipher the key should be periodically dropped after no access for 5 sec and re-read from disc during encryption decryption when next required.


