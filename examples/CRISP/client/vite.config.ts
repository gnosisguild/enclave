// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'
import svgr from '@svgr/rollup'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import path from 'path'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

// const development: boolean = !process.env.NODE_ENV || process.env.NODE_ENV === 'development'

export default defineConfig({
  base: '/',
  define: {
    // here is the main update
    global: 'globalThis',
  },
  optimizeDeps: {
    esbuildOptions: { target: "esnext" },
    exclude: ['@rollup/browser', '@noir-lang/noirc_abi', '@noir-lang/acvm_js', '@enclave-e3/wasm', '@enclave-e3/wasm/init'],
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      libs: path.resolve(__dirname, './libs'),
    },
  },
  worker: {
    format: 'es',
  },
  plugins: [
    // here is the main update
    wasm(),
    topLevelAwait(),
    react({
      jsxImportSource: '@emotion/react',
      babel: {
        plugins: ['@emotion/babel-plugin'],
      },
    }),
    viteTsconfigPaths(),
    svgr(),
    nodePolyfills({ include: ['buffer'] }),
  ],
  server: {
    open: true,
    // this sets a default port to 3000
    port: 3000,
  },
})
