# e3-support-scripts-dev

Development runner template for an application's E3 program. It runs the project's `fhe_processor`
and policy natively and returns an ABI-shaped **mock proof**, not a verified zkVM receipt. It must
be used only with an explicitly unproved development setup.

## Run in a generated project

This package is excluded from the repository's root workspace.
[interfold init](../../init/README.md) copies/links it under the project's `.interfold/support/dev`,
where its `e3-user-program` relative dependency resolves. Do not run root
`cargo -p e3-support-scripts-dev` here.

From that prepared project's root:

```sh
interfold compile --dev true
interfold program start --dev true
```

The parent [dispatcher](../README.md) invokes the copied support scripts. For direct package checks
in the generated workspace use `cargo test -p e3-support-scripts-dev` or
`cargo build --bin e3-support-scripts-dev`; the first copied-project build may update lockfile
sources.

## HTTP behavior

The runner uses [E3ProgramServer](../../program-server/README.md), including its `POST /run_compute`
request/callback boundary. `ComputeManager::with_published` retains published input metadata, and
the same caller-selected policy is passed through computation and mock proving. An ABI-compatible
envelope is not cryptographic verification.

The server has no caller authentication or durable job queue. Use the
[OpenVM service](../openvm/service/README.md) and prepared worker artifacts when real receipt
verification is required; this runner is never an automatic proving-failure fallback.
