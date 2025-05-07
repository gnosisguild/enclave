import { test, expect } from "@playwright/test";

test("CRISP smoke test", async ({ page }) => {
  // Create a simple Hello World page for testing
  // await page.setContent("<h1>Hello World!</h1>");

  // Test that the page contains the text "Hello World!"
  const heading = page.locator("h4");
  await expect(heading).toHaveText("Coercion-Resistant Impartial Selection Protocol");

  // Take a screenshot
  await page.screenshot({ path: "./test-results/smoke.png" });
});
