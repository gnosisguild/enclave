# `user_data_encryption_ct1` — P3 (standalone leg)

Second leg of user BFV encryption, as a standalone circuit.
[`user_data_encryption`](../user_data_encryption/README.md) proves this leg and
`user_data_encryption_ct0` together, and clients prove only that circuit. This package builds for
per-leg benchmarks.

|           |                                                                                                                     |
| --------- | ------------------------------------------------------------------------------------------------------------------- |
| **Core**  | [`lib/src/core/threshold/user_data_encryption_ct1.nr`](../../../lib/src/core/threshold/user_data_encryption_ct1.nr) |
| **Index** | [Circuit package index](../../../README.md#circuit-package-index)                                                   |
| **Docs**  | [Noir Circuits](../../../../docs/pages/noir-circuits.mdx)                                                           |
