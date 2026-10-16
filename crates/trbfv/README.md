# e3-trbfv

Threshold-BFV calculations and typed request/response payloads for distributed key generation and
threshold decryption. Actors, authenticated dealer admission, and timing belong in
[keyshare](../keyshare/README.md) and [aggregation](../aggregator/README.md).

## Calculation surface

Each public module exposes a typed request/response and its calculation function:

| Module/function                  | Role                                                    |
| -------------------------------- | ------------------------------------------------------- |
| `gen_pk_share_and_sk_sss`        | Public-key contribution and secret-key Shamir shares    |
| `gen_esi_sss`                    | Smudging/error-related Shamir material                  |
| `calculate_decryption_key`       | Assemble this node's decryption key from dealer shares  |
| `calculate_decryption_share`     | Produce this node's share for a ciphertext              |
| `calculate_threshold_decryption` | Reconstruct plaintext from sufficient decryption shares |

`TrBFVRequest`, `TrBFVResponse`, and `TrBFVError` provide the worker dispatch envelope.
`TrBFVFailure` retains structured threshold error/party attribution when available, in addition to a
human-readable message. The [multithread dispatcher](../multithread/src/multithread.rs) is the
production caller showing RNG/cipher/request handling.

`TrBFVConfig::new(encoded_params, num_parties, threshold)` receives T, not T+1. Runtime `PartyId` is
zero-based; Shamir reconstruction coordinates are one-based at the mathematical boundary. Share
ordering must retain the corresponding party IDs. Use paired threshold/DKG metadata from
[e3-fhe-params](../fhe-params/README.md), not interchangeable parameter sets.

## Secrets and tests

Share payloads in `shares` use local encrypted wrappers. Their opaque serialized bytes do not make
decoded secret material suitable for logging or ordinary snapshots. Randomized operations take
caller RNG/cipher inputs; runtime recovery reuses successful outputs rather than silently generating
different shares.

Run `cargo test --locked -p e3-trbfv` with [test-helper prerequisites](../test-helpers/README.md).
See [DKG](../../docs/architecture/dkg.md) and
[threshold decryption](../../docs/architecture/threshold-decryption.md) for dimensions and proofs.
