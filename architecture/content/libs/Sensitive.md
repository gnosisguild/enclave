---
type: lib
description: An Arc clonable data container where it's contents are stored encrypted and decrypted on access to a zeroizable vector.
---
## `=this.file.name`

`=this.description`

#### Description

We need to be able to send sensitive data in messages. This is a container that allows access to it's contents by calling the `fn access(&self) -> Zeroizable<T>` method where the internal value is encrypted using the [[Cipher]]  and decrypted on access. The internal data is stored in an `Arc` so that there is only a single place for the data in memory.