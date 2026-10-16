# e3-support-scripts

## Purpose

Project compile/start dispatcher used by the Interfold CLI. It invokes the consuming project's
`.interfold/support` scripts; the Rust runner sources here are templates, not programs that can be
run directly from this repository's root workspace.

## Map

| Location                                                                                     | Responsibility                                                              |
| -------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| [`src/lib.rs`](src/lib.rs), [`src/program.rs`](src/program.rs)                               | Dispatch API, supported-mode validation, and explicit dev/OpenVM selection. |
| [`src/program_openvm.rs`](src/program_openvm.rs), [`src/program_dev.rs`](src/program_dev.rs) | Backend script paths/environment.                                           |
| [`src/utils.rs`](src/utils.rs)                                                               | Child invocation and safe failure reporting.                                |
| [`dev/`](dev), [`openvm/`](openvm)                                                           | Project templates: compile/start scripts and Rust runners.                  |

The CLI passes resolved `ProgramConfig`; this library selects a backend and invokes the consuming
project's support script. That script builds/starts the project-specific runner, which owns jobs.
The root library is not itself the compute service.

## Project commands

From a project created with [interfold init](../init/README.md):

```sh
interfold compile
interfold program start
```

OpenVM is the default backend. Configure absolute CPU/CUDA worker and optional setup paths under
`program.openvm`; the dispatcher forwards the selected backend and paths to compile/start. Prepare
worker tooling, guest configuration, and keys with the [OpenVM guide](../openvm-prover/README.md).

Unproved development execution requires an explicit choice, for example
`interfold compile --dev true` and `interfold program start --dev true`, or `program.dev`
configuration. It uses the [development runner](dev/README.md), not a verified receipt. The
[OpenVM service](openvm/service/README.md) runs real proofs; a resource failure does not implicitly
select development mode.

## Library API and layout

`program_compile(ProgramConfig, Option<bool>)` and `program_start(...)` are async entry points. The
optional boolean overrides configured development mode. Unsupported backend configuration is
rejected before dispatch. `program_cache_purge()` deletes the current project's `.interfold/caches`;
it is a destructive cache operation, not node-state maintenance.

`interfold init` rewrites dependencies when copying/linking support into a generated project. The
child `dev` and `openvm/service` packages are excluded from the root workspace and resolve
`e3-user-program` relative to the generated project. Its first build may refresh copied lockfile
sources.

## Tests

Run `cargo test --locked -p e3-support-scripts`. Inline `program.rs` cases pin default OpenVM and
explicit dev selection; `utils.rs` checks environment forwarding and failure output without leaking
arguments. Runner integration needs a prepared disposable project; package checks are in the child
READMEs above. [AGENTS.md](AGENTS.md) covers dispatch/template changes.
