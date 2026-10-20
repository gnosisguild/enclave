# OpenVM repository tasks

## Purpose

`pnpm openvm <task> [arguments]` dispatches cross-workspace CLI, prover and service tasks, plus
CRISP development scenarios, forwarding arguments and exit status. Run without a task for the task
list. CRISP's source and scenario ownership remain in `examples/CRISP`; projects created with
`interfold init` use `interfold program compile` and `interfold program start` instead.

## Map

- [`run.sh`](run.sh): dispatches commands to the appropriate Rust workspace or CRISP scenario.
- [`run.test.mjs`](run.test.mjs): tests arguments, target-directory overrides, and exit propagation.
- [`provenance.ts`](provenance.ts): records artifact, worker, and deployed receipt identity.

## Setup and usage

Follow the [OpenVM worker setup](../../crates/openvm-prover/README.md) for pinned tooling and
artifacts; [CRISP setup](../../examples/CRISP/Readme.md) covers its linked service/project. For
example, `pnpm openvm prover-check` checks the independent worker workspace, while
`pnpm openvm service-check` checks the CRISP project's service. The root Node dependencies are
required for provenance's contract/RPC helpers.

## Guest provenance

`pnpm provenance:manifest` records the source commit, lockfile digests, guest configuration, proving
artifacts, KZG parameters, and application commitments. With `--prover`, it checks the worker's
configured identity. With an RPC, it checks protocol-to-receipt binding, application commitments,
and deployed Halo2 runtime against the checked artifact, without sending transactions.

```sh
pnpm provenance:manifest
pnpm provenance:manifest --config <worker.json> --prover <worker-binary> --rpc <url> --verifier <protocol-verifier-address> --out manifest.json
```

Missing fields produce `"complete": false`; a release manifest must be complete. Artifact digests,
application commitments, and receipt identity establish different bindings, not source
reproducibility or audit coverage. Follow the full
[verification procedure](../../docs/pages/build/e3-program/verify-compute-provider.mdx).

## Tests

`pnpm test:scripts` checks dispatch and argument/exit-status forwarding without compiling a guest.
The `guest-parity`, `proof-test`, and `service-e2e` tasks exercise different execution/acceptance
boundaries and require the prepared tools and services described by the worker and CRISP guides.
