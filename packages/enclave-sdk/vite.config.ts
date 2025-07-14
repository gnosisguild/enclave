
import { defineConfig } from "vite";
import dts from "vite-plugin-dts";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";
import { resolve } from "path";
export default defineConfig({
  base: "./",
  build: {
    minify: false,
    lib: {
      entry: resolve(__dirname, "src/index.ts"),
      name: "EnclaveClient",
      formats: ["es"],
      fileName: (format) => `index.${format}.js`,
    },
    sourcemap: false,
  },
  worker: {
    format: "es",
    plugins: () => [wasm(), topLevelAwait(), dts({ rollupTypes: true })],
  },
  plugins: [wasm(), topLevelAwait()],
});
