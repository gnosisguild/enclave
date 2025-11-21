import { defineConfig, globalIgnores } from 'eslint/config'
import config from '@enclave-e3/config/eslint.config.js'

export default defineConfig([
  globalIgnores([
    // Github submodules.
    'lib/risc0-ethereum',
    // Build and cache directories.
    '**/node_modules/**',
    '**/dist/**',
    '**/build/**',
    '**/cache/**',
    '**/coverage/**',
    '**/target/**',
    '**/artifacts/**',
    '**/types/**',
    '**/deployments/**',
  ]),
  {
    extends: [config],
    files: ['**/*.{ts,tsx,js,jsx}'],
  },
])
