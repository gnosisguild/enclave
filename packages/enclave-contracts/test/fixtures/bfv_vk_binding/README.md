# BFV VK binding fixtures

Golden `dkg_aggregator` / `decryption_aggregator` EVM proofs for
`test/BfvVkBindingIntegration.spec.ts`. Independent of `circuits/benchmarks/`.

Refresh after circuit or aggregator public-input layout changes:

```bash
# From repo root, after insecure integration run exports integration_summary:
jq '.folded_artifacts' circuits/benchmarks/results_insecure/integration_summary.json \
  > packages/enclave-contracts/test/fixtures/bfv_vk_binding/folded_artifacts.json
```

Or set `BFV_VK_BINDING_FOLDED_ARTIFACTS` to another JSON file with the same
shape.
