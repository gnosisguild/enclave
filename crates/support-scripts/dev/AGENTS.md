# e3-support-scripts-dev

This excluded template runs a consuming project's processor with a mock proof, never real zkVM
verification. It becomes a member of that project's workspace after copying/linking.

- The `e3-user-program` relative path and inherited dependencies are resolved in the project layout;
  root `cargo -p e3-support-scripts-dev` is not a valid repository check.
- Preserve `with_published` and pass one caller-selected policy through computation and proving.
  Dropping metadata changes application leaves even though the development seal is mocked.
- Keep the mock envelope ABI-compatible with the current Solidity consumer; a valid envelope is not
  a cryptographic receipt. Do not let this provider become a resource/error fallback for OpenVM.
- Change template sources here and exercise a copied/linked project. Do not fix relative imports by
  hard-coding this repository's local filesystem paths.

Use the [README project-context tests](README.md#tests). Exercise changed processor/policy plumbing
in a generated project, not just the mock envelope unit case. See the
[parent guidance](../AGENTS.md) for dispatch and
[program-server guidance](../../program-server/AGENTS.md) for the shared HTTP boundary.
