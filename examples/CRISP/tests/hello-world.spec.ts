import { test, expect } from "@playwright/test";

test("Hello World test", async ({ page }) => {
  // Create a simple Hello World page for testing
  await page.setContent("<h1>Hello World!</h1>");

  // Test that the page contains the text "Hello World!"
  const heading = page.locator("h1");
  await expect(heading).toHaveText("Hello World!");

  // Take a screenshot
  await page.screenshot({ path: "./test-results/hello-world.png" });
});
