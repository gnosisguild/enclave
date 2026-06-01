// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './test',
  timeout: 5 * 60 * 10000,
  use: {
    baseURL: 'http://localhost:3000',
    actionTimeout: 75 * 1000,
    // bb.js fetches the public Aztec CRS over HTTPS from the page. On CI the
    // runner's browser rejects the served cert as expired
    // (ERR_CERT_DATE_INVALID) even though it succeeds locally. The CRS is
    // public, integrity-checked data, so ignore cert errors for the e2e run.
    ignoreHTTPSErrors: true,
  },
  retries: 0,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  workers: process.env.CI ? 1 : undefined,
  // reporter: "html",
  reporter: [['html'], ['list']], // Add list reporter

  // Add support for ES modules
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        headless: true,
        // ignoreHTTPSErrors alone can miss cross-origin sub-resource fetches
        // (the CRS download); the Chromium flag covers all cert errors.
        launchOptions: {
          args: ['--ignore-certificate-errors'],
        },
      },
    },
  ],
})
