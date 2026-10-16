# Repository scripts

## Purpose

Root [`package.json`](../package.json) supplies supported commands; implementations and tests live
with the capability they maintain.

## Map

| Owner                             | Commands and usage                                                                                                            |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| [checks/](checks/README.md)       | `pnpm check:repository`, `check:committee`, `check:addresses`, `check:bindings`, `check:license`, `check:pnpm`, `check:shell` |
| [circuits/](circuits/README.md)   | `pnpm build:circuits`, `store:circuits`, `generate:verifiers`, `check:verifiers`, `noir:lint`, `noir:test`                    |
| [release/](release/README.md)     | `pnpm bump:versions` prepares files by default; commit/push are explicit. `pnpm release:tag` creates and pushes a tag.        |
| [dev/](dev/README.md)             | `pnpm clean` removes rebuildable output, never operator state or indexed files.                                               |
| [openvm/](openvm/README.md)       | `pnpm openvm <task>`, `pnpm provenance:manifest`                                                                              |
| [hooks/](hooks/README.md)         | Shared Git hook helpers; `.husky/` owns hook entry points.                                                                    |
| [blueprint/](blueprint/README.md) | Architecture map, navigation checks and PR previews.                                                                          |

Application scenarios remain with their application: CRISP's
[`test-isolated.sh`](../examples/CRISP/scripts/test-isolated.sh) tests committed `HEAD` in a
temporary worktree and rejects pending changes. Failed runs retain their worktree for diagnosis.
Network runner regressions live beside [`crates/net/tests/run.sh`](../crates/net/tests/run.sh).

## Tests

Focused verification: `pnpm test:scripts`, `pnpm test:release`, `pnpm test:circuit-tooling`, and
`pnpm test:harnesses`. These tooling tests do not prove cryptographic soundness or qualify a
release.

Contract-size checks and sale rehearsals belong to the
[contract package](../packages/interfold-contracts/README.md). [AGENTS.md](AGENTS.md) describes
conventions for changing repository tooling.
