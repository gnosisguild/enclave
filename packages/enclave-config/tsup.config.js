import { defineConfig } from "tsup";

export const baseConfig = defineConfig({
  entry: ["src/index.ts"],
  splitting: false,
  sourcemap: true,
  clean: true,
  format: ["esm"],
  dts: true,
});
