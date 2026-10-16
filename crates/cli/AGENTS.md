# e3-cli

Keep argument parsing, prompts, and operator output here; delegate node lifecycle and protocol work
to their existing owners rather than implementing another startup path.

- Contract calls generate Rust bindings from tracked contract artifact JSON. After contract API
  changes run `pnpm evm:build` and `pnpm check:bindings`; do not patch a generated ABI to hide a
  mismatch. The [binding check](../../scripts/checks/cli-bindings.ts) documents this boundary.
- `build.rs` reads `packages/interfold-contracts/deployed_contracts.json` and generates Rust data in
  `OUT_DIR`. Change deployment records through the contract tooling, not generated Rust. `GIT_SHA`
  overrides build identity; without local Git metadata the build may query the remote.
- Preserve explicit confirmation and stopped-node safety for reset/purge/update commands. A
  successful prompt is not a replacement for entrypoint's process fence and state checks.
- Keep proof-skip arguments behind the dedicated test-only feature; never make missing proof
  artifacts silently choose an unproved production mode.

Use the [README tests/build prerequisites](README.md#tests). Command changes need the affected
executable/daemon path, not only parser assertions. Contract API changes also need the binding check
above; generated data is not the source of truth.
