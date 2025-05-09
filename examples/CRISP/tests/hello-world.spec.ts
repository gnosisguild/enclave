import { test, expect } from "@playwright/test";

test("CRISP smoke test", async ({ page }) => {
  await page.goto("http://localhost:3000");
  await expect(page.locator("h4")).toHaveText(
    "Coercion-Resistant Impartial Selection Protocol",
  );
});
