// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import path from 'path'

export default defineConfig({
  base: '/',
  define: {
    global: 'globalThis',
  },
  optimizeDeps: {
    exclude: ['@rollup/browser', '@enclave-e3/wasm'],
  },
  build: {
    commonjsOptions: {
      include: [/node_modules/, /packages\/evm/],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      libs: path.resolve(__dirname, './libs'),
    },
  },
  plugins: [wasm(), topLevelAwait(), react(), viteTsconfigPaths()],
  server: {
    open: true,
    port: 3000,
  },
  preview: {
    port: 3000,
    open: true,
  },
})
