# e3-support-scripts-openvm

This excluded template wires the project's processor/policy into the HTTP service and native OpenVM
host. It runs in the consuming project's workspace, not the root Rust workspace.

- Preserve the full request computation domain when constructing `ComputeDomain`; omitting fields
  weakens the journal's replay binding even when native ciphertext computation still succeeds.
- Start/check the prover before accepting jobs. Backend selection and timeouts belong to
  `WorkerConfig`; the service must not replace failed proofs with development output.
- The user-program relative path is intentionally project-layout dependent. Change the template,
  then exercise a copied/linked project instead of rewriting it for root `cargo -p` convenience.
- Preserve explicit request/concurrency/body limits and loopback defaults. The HTTP boundary has no
  authentication or durable queue; a real runner does not add those properties.

Use the [README project checks/prerequisites](README.md#tests-and-project-checks). Domain/runner
changes require prepared service/contract acceptance as well as checking the linked package;
compilation alone cannot establish proof identity.
