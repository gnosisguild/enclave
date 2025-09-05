import { baseConfig } from "@enclave-e3/config/tsup";
import { defineConfig } from "tsup";

export default defineConfig({
  ...baseConfig,
  entry: ["types/index.ts", "scripts/deployEnclave.ts", "scripts/deployMocks.ts"],
  include: ["hardhat.config.ts", "scripts/**/*.ts"],
  // include: ["./scripts/**/*.ts", "./types/**/*.ts", "./tasks/**/*.ts", "./ignition/modules/*.ts"],
  // entry: ["scripts/deployEnclave.ts", "scripts/deployMocks.ts", "types/index.ts"],
  // lib: ["es2023"],
  // module: "node16"
});
