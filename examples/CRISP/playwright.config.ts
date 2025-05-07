import { defineConfig } from '@playwright/test';
import { execSync } from 'child_process';

// Use system browser when PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH is set
const systemBrowser = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH ||
  (process.env.PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS ?
    execSync('which chromium').toString().trim() :
    undefined);

export default defineConfig({
  testDir: './tests',
  timeout: 30000,
  use: systemBrowser ? {
    // Use chrome channel and system browser to avoid headless_shell issues
    channel: 'chrome',
    launchOptions: { executablePath: systemBrowser }
  } : {},
  projects: [{ name: 'chromium', use: {} }],
  retries: process.env.CI ? 2 : 0,
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  workers: process.env.CI ? 1 : undefined,
  reporter: 'html',
});
