import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./test",
  timeout: 10 * 60 * 1000,
  use: {
    baseURL: "http://localhost:3000",
  },
  retries: 0,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  workers: process.env.CI ? 1 : undefined,
  reporter: "html",
});
