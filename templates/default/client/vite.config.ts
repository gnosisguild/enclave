import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import viteTsconfigPaths from 'vite-tsconfig-paths'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import path from 'path'

// const development: boolean = !process.env.NODE_ENV || process.env.NODE_ENV === 'development'

export default defineConfig({
  base: '/',
  define: {
    // here is the main update
    global: 'globalThis',
  },
  optimizeDeps: {
    exclude: ['@rollup/browser'],
    include: ['@gnosis-guild/enclave-sdk', '@gnosis-guild/enclave-react'],
    force: true,
  },
  build: {
    commonjsOptions: {
      include: [/node_modules/, /packages\/evm/],
    },
  },
  resolve: {
    alias: {
      react: path.resolve(__dirname, 'node_modules/react'),
      'react-dom': path.resolve(__dirname, 'node_modules/react-dom'),
      wagmi: path.resolve(__dirname, 'node_modules/wagmi'),
      '@gnosis-guild/enclave-react': path.resolve(__dirname, 'node_modules/@gnosis-guild/enclave-react'),
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
