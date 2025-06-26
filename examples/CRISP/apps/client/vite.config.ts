import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'
import wasm from 'vite-plugin-wasm'
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
    exclude: ['@rollup/browser', '@noir-lang/noirc_abi', '@noir-lang/acvm_js'],
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      libs: path.resolve(__dirname, './libs'),
    },
  },
  plugins: [
    // here is the main update
    wasm(),
    react({
      jsxImportSource: '@emotion/react',
      babel: {
        plugins: ['@emotion/babel-plugin'],
      },
    }),
    viteTsconfigPaths(),
    nodePolyfills({ include: ['buffer'] }),
  ],
  server: {
    open: true,
    // this sets a default port to 3000
    port: 3000,
  },
})
