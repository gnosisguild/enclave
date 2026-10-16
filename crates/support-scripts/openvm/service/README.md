# e3-support-scripts-openvm

## Purpose

Project-local HTTP runner connecting an application's `fhe_processor`/policy to the native
[OpenVM host](../../../openvm-host/README.md). It returns a real worker-verified proof envelope and
ciphertext through [E3ProgramServer](../../../program-server/README.md).

## Map and startup flow

[`src/main.rs`](src/main.rs) is the composition root: resolve configuration, start/check `Prover`,
configure HTTP limits, and translate each job's full domain into the host call. The project supplies
`e3_user_program::fhe_processor` and `policy`; the host supplies execution/proof verification.
Sibling support scripts build/configure these project-specific pieces.

## Project setup

This template is excluded from the root Rust workspace. [interfold init](../../../init/README.md)
copies/links it into the generated project's `.interfold/support/openvm/service`; its relative
`e3-user-program` dependency is meaningful there, not in a root `cargo -p` command here.

Prepare the pinned [OpenVM tooling, artifacts, and keys](../../../openvm-prover/README.md),
configure `program.openvm` with absolute worker/setup paths, then run from the generated project:

```sh
interfold compile
interfold program start
```

The support scripts set `OPENVM_PROVER_CONFIG` and worker environment. `Prover::start` checks the
selected worker/artifacts before the server accepts computation jobs. Auto CPU/CUDA selection and
per-operation deadlines are configured through `WorkerConfig`; failed proofs do not become
development output.

## Service configuration

- `OPENVM_BIND_ADDR`: default `127.0.0.1:13151`.
- `MAX_CONCURRENT_COMPUTATIONS`: default 1.
- `OPENVM_MAX_REQUEST_BYTES`: default 128 MiB.
- `OPENVM_BODY_TIMEOUT_SECS`: default 120 seconds.

Each request supplies the full chain/deployment/E3/encryption-scheme/committee-key `ComputeDomain`.
The HTTP API has no authentication or durable job queue; keep the loopback default unless a separate
access-control boundary is provided.

## Tests and project checks

Use root `pnpm openvm service-check` for the linked CRISP project, or
`cargo check -p e3-support-scripts-openvm` in a prepared generated workspace. Its first build may
refresh copied lockfile sources; successful checking does not prove a computation.

The template has no standalone test functions. Root `pnpm openvm service-test` exercises
host/types/Secure Process/HTTP owners; `pnpm openvm service-e2e` runs the prepared CRISP
service/contract flow described in the [OpenVM guide](../../../openvm-prover/README.md). See
[AGENTS.md](AGENTS.md) for wiring changes.
