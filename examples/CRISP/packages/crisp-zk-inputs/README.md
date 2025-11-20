# Wasm bundle for crisp-zk-inputs

Here we export wasm functionality for consumption in TypeScript to enable us to share code between
Rust and TypeScript.

## Usage

This package exposes an `init` subpackage default function which should be used to universally load
the wasm module instead of exporting the default loader.

This is because in modern node there is no need for preloading however in the browser we still need
to load the wasm bundle.

### ❌ DONT USE THE DEFAULT INIT

```ts
// Bad! Because this uses the raw loader which doesn't exist in node contexts
import init, { bfvEncryptNumber } from '@crisp-e3/zk-inputs'
```

### ✅ DO USE THE EXPORTED SUBMODULE

```ts
// Good! Use the universal loader
import init from '@crisp-e3/zk-inputs/init'

await init()
// other package imports here
```
