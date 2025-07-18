import { baseConfig } from "@gnosis-guild/enclave-config/tsup";
import { defineConfig } from "tsup";

export default defineConfig({
  ...baseConfig,
  entry: ["deploy/enclave.ts", "deploy/mocks.ts", "types/index.ts"],
  include: ["./deploy/**/*.ts", "./types/**/*.ts"],
  external: [/^mocha/, /^ts-node/],
  format: ["esm", "cjs"],
});
