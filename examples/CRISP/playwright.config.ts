// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./test",
  timeout: 5 * 60 * 1000,
  use: {
    baseURL: "http://localhost:3000",
    actionTimeout: 60 * 1000,
  },
  retries: process.env.CI ? 2 : 0,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
});
