// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { defineConfig } from "tsup";
import { baseConfig } from "@enclave-e3/config/tsup";

export default defineConfig([
  {
    ...baseConfig,
    include: ["./src/**/*.ts"],
    format: ["esm"],
    outExtension: () => ({
      js: ".js",
    }),
    esbuildOptions: (options) => {
      options.alias = {
        "@enclave-e3/wasm/init": "../../crates/wasm/init_node.js",
      };
    },
  },
  {
    ...baseConfig,
    include: ["./src/**/*.ts"],
    format: ["cjs"],
    outExtension: () => ({
      js: ".cjs",
    }),
    esbuildOptions: (options) => {
      options.alias = {
        "@enclave-e3/wasm/init": "../../crates/wasm/init_node.cjs",
      };
    },
  },
]);
