# `user_data_encryption_ct0` — P3 (standalone leg)

First leg of user BFV encryption under the **aggregated** threshold public key, as a standalone
circuit. [`user_data_encryption`](../user_data_encryption/README.md) proves this leg and
`user_data_encryption_ct1` together, and clients prove only that circuit. This package builds for
per-leg benchmarks and for the SDK proof test, which derives `k1_commitment` from it.

|           |                                                                                                                     |
| --------- | ------------------------------------------------------------------------------------------------------------------- |
| **Core**  | [`lib/src/core/threshold/user_data_encryption_ct0.nr`](../../../lib/src/core/threshold/user_data_encryption_ct0.nr) |
| **Index** | [Circuit package index](../../../README.md#circuit-package-index)                                                   |
| **Docs**  | [Noir Circuits](../../../../docs/pages/noir-circuits.mdx)                                                           |
