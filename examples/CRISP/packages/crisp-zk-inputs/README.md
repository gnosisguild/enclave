# @crisp-e3/zk-inputs

WASM bindings for generating CRISP ZK proof inputs, compiled from Rust and shared between the
server-side Node.js environment and the browser. This package lets the CRISP SDK produce the circuit
witness data needed for Noir-based vote-validity proofs without duplicating the logic in TypeScript.

## What it generates

The WASM module exposes functions for computing ZK circuit inputs for CRISP's two Noir circuits:

- **Vote proof inputs** — proves a vote was cast by an eligible participant with a valid Merkle
  membership witness.
- **Masked vote proof inputs** — same as above but with an additional blinding factor for additional
  privacy.

These inputs are then passed to `@noir-lang/noir_js` and `@aztec/bb.js` to generate the actual
proofs.

## Usage

This package requires a universal init pattern because:

- In **Node.js** (>=18) WASM can be loaded synchronously — no preloading needed.
- In the **browser** the WASM binary must be fetched and instantiated asynchronously.

The `init` subpackage handles both environments transparently.

### ❌ Don't use the default export

```ts
// Bad — the raw default loader doesn't work in Node.js contexts
import init, { generateVoteInputs } from '@crisp-e3/zk-inputs'
```

### ✅ Use the universal subpackage loader

```ts
import init from '@crisp-e3/zk-inputs/init'
import { generateVoteInputs } from '@crisp-e3/zk-inputs'

await init()
const inputs = generateVoteInputs(/* ... */)
```

Call `init()` once before any other imports from `@crisp-e3/zk-inputs`. In browser environments
`init()` fetches the WASM binary; in Node.js it is a no-op.

## Building

The WASM bundle is compiled from the Rust source in `crates/crisp-zk-inputs` using `wasm-pack`:

```bash
# From the CRISP root
pnpm build:wasm
```
