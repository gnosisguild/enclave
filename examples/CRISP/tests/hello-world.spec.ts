import { test, expect } from "@playwright/test";

test("CRISP smoke test", async ({ page }) => {
  // Navigate to the application
  await page.goto("http://localhost:3000");

  // Verify the heading exists with the correct text
  await expect(page.locator("h4")).toHaveText("Coercion-Resistant Impartial Selection Protocol");

  // Take a screenshot
  await page.screenshot({ path: "./test-results/smoke.png" });
});
