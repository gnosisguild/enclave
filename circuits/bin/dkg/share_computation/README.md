# `share_computation` — C2 (final wrapper)

Verifies **`N_BATCHES`** inner batch proofs (non-ZK UltraHonk verify), folds their commitments, and
checks the VK genealogy (`key_hash`). This is the **ProofType C2a / C2b** surface circuit — upstream
packages are `sk_share_computation_base` / `e_sm_share_computation_base`, `share_computation_chunk`,
`share_computation_chunk_batch`.

|           |                                                                                       |
| --------- | ------------------------------------------------------------------------------------- |
| **Core**  | [`lib/src/core/dkg/share_computation/`](../../../lib/src/core/dkg/share_computation/) |
| **Index** | [Circuit package index](../../../README.md#circuit-package-index)                     |
| **Docs**  | [Noir Circuits](../../../../docs/pages/noir-circuits.mdx)                             |
