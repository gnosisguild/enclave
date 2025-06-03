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
    topLevelAwait(),
    react(),
    viteTsconfigPaths(),
  ],
  server: {
    open: true,
    // this sets a default port to 3000
    port: 3000,
  },
  preview: {
    port: 3000,
    open: true,
  },
})
