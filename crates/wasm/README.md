# Wasm bundle for enclave

Here we export wasm funcionality for consumption in typescript to enable use to share code between Rust and Typescript.

## Usage

This package exposes an `init` subpackage default function which should be used to universally load the wasm module instead of exporting the default loader.

This is because in modern node there is no need for preloading however in the browser we still need to load the wasm bundle.

##### ❌ DONT USE THE DEFAULT INIT

```ts
// Bad! Because this uses the raw loader which doesn't exist in node contexts
import init, { encrypt_number } from "@gnosis-guild/e3-wasm";
```

##### ✅ DO USE THE EXPORTED SUBMODULE

```ts
// Good! Use the universal loader
import init from "@gnosis-guild/e3-wasm/init";
import { encrypt_number } from "@gnosis-guild/e3-wasm";

export async function encryptNumber(
  data: bigint,
  public_key: Uint8Array,
): Promise<Uint8Array> {
  await init();
  return encrypt_number(data, public_key);
}
```
