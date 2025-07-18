import { defineConfig } from "tsup";
import { baseConfig } from "@gnosis-guild/enclave-config/tsup";

export default defineConfig({
  ...baseConfig,
  include: ["./src/**/*.ts"],
});
