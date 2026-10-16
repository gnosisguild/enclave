# e3-slashing

Ciphernode accusation admission/voting and cross-proof commitment consistency. This capability
authenticates evidence and forms votes; the EVM adapter submits accepted quorum outcomes and
contracts settle them. `e3-zk-prover` retains compatibility re-exports, not another implementation.

## Router extensions

Register these per-E3 capabilities through `E3RouterBuilder::with_recipient`:

- `AccusationManagerExtension::create(bus, signer, slashing_managers_by_chain, vote_validity_secs_by_chain, deadline_skew_secs, persisted_committees)`
  creates/hydrates the `accusation_manager` recipient. Manager addresses bind signatures to the
  correct chain's SlashingManager; committees supply canonical party attribution.
- `CommitmentConsistencyCheckerExtension::create(bus, store, links_factory)` supplies the
  `commitment_consistency_checker` recipient. The factory maps a BFV preset to the expected
  `CommitmentLink` rules; the standard composition uses `e3_zk_prover::default_links`.

The [builder caller](../ciphernode-builder/src/builder/extensions.rs) shows chain-specific
configuration and registration. Exported `AccusationManager` and `CommitmentConsistencyChecker` also
support lower-level harnesses; extensions are the normal runtime entry.

## Evidence boundary

Authentication, request/party attribution, proof type, and signed windows precede evidence/vote
admission. Local process, I/O, and memory failures are not invalid-peer proofs. Quorum agrees on a
specific preimage, not just any collection of accusation labels. Restoring an actor does not
reconstruct all volatile votes or timers.

The [slashing domain](../../docs/architecture/slashing.md) documents accusation lanes, quorum, and
settlement dependencies; [DKG](../../docs/architecture/dkg.md) owns commitment links. Run
`cargo test --locked -p e3-slashing` for local coverage. Changed proof-to-slash behavior also needs
prepared `pnpm rust:test:slashing` with the [prover prerequisites](../zk-prover/README.md).
