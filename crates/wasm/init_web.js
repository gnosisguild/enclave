// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import * as bindgen from "./dist/web/e3_wasm.js";

let promise;

export default async function initializeWasm(initParams) {
  promise ??= (async () => {
    const { default: base64 } = await import("./dist/web/e3_wasm_base64.js");

    const binaryString = atob(base64);
    const len = binaryString.length;
    const bytes = new Uint8Array(len);

    for (let i = 0; i < len; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }

    bindgen.initSync(bytes);

    return bindgen;
  })();

  return promise;
}
