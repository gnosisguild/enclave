import { defineConfig } from 'vitest/config'
import wasm from 'vite-plugin-wasm'

export default defineConfig({
  test: {
    environment: 'node',
  },
  plugins: [wasm()],
})
