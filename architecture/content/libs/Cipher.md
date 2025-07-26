---
type: lib
source: https://github.com/gnosisguild/enclave/blob/main/crates/crypto/src/cipher.rs
tags:
  - todo
  - trbfv
---
The cipher is a library that encrypts data to the key found in the keyfile the location of which is configured within the `enclave.config.yaml` 

Additions we should consider with the cipher:
- Currently the key remains in the cipher the key should be periodically dropped after no access for 5 sec and re-read from disc during encryption decryption when next required.


