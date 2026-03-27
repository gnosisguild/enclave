# `fold`

Pairwise aggregation of **two** non-ZK UltraHonk proofs: verifies each under its VK, merges
commitments, and computes a **genealogy** `key_hash` over inner key hashes and VK hashes.

|              |                                                                   |
| ------------ | ----------------------------------------------------------------- |
| **Source**   | [`src/main.nr`](src/main.nr)                                      |
| **Wrappers** | [../wrapper/README.md](../wrapper/README.md)                      |
| **Index**    | [Circuit package index](../../../README.md#circuit-package-index) |
| **Docs**     | [Noir Circuits](../../../../docs/pages/noir-circuits.mdx)         |

Output: `pub (Field, Field)` = `(key_hash, commitment)`.
