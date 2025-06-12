import { Page } from "@playwright/test";
import { testWithSynpress } from "@synthetixio/synpress";
import { MetaMask, metaMaskFixtures } from "@synthetixio/synpress/playwright";
import basicSetup from "./wallet-setup/basic.setup";
import { execSync } from "child_process";

console.log("📄 [TEST FILE LOADED]");

async function runCliInit() {
  try {
    console.log("🚀 [runCliInit] Starting CLI init...");
    const output = execSync("pnpm cli init", { encoding: "utf-8" });
    console.log("✅ [runCliInit] Command output:\n", output);
    return output;
  } catch (error) {
    console.error("❌ [runCliInit] CLI init failed:", error);
    throw error;
  }
}

const test = testWithSynpress(metaMaskFixtures(basicSetup));
const { expect } = test;

async function ensureHomePageLoaded(page: Page) {
  console.log("🔍 [ensureHomePageLoaded] Verifying homepage text...");
  return await expect(page.locator("h4")).toHaveText(
    "Coercion-Resistant Impartial Selection Protocol",
  );
}

test("CRISP smoke test", async ({
  context,
  page,
  metamaskPage,
  extensionId,
}) => {
  console.log("🧪 [TEST START] CRISP smoke test");

  const metamask = new MetaMask(
    context,
    metamaskPage,
    basicSetup.walletPassword,
    extensionId,
  );

  console.log("🛠️ [Step] Running CLI init");
  await runCliInit();

  console.log("🌐 [Step] Navigating to home page");
  await page.goto("/");

  console.log("📄 [Step] Ensuring home page is loaded");
  await ensureHomePageLoaded(page);

  console.log("🔗 [Step] Connecting Wallet");
  await page.locator('button:has-text("Connect Wallet")').click();
  await page.locator('button:has-text("MetaMask")').click();
  await metamask.connectToDapp();

  console.log("🎮 [Step] Trying Demo");
  await page.locator('button:has-text("Try Demo")').click();

  console.log("🗳️ [Step] Selecting poll");
  await page
    .locator("[data-test-id='poll-button-0'] > [data-test-id='card']")
    .click();

  console.log("🗳️ [Step] Casting vote - part 1");
  await page.locator('button:has-text("Cast Vote")').click();

  console.log("🧾 [Step] Registering identity");
  await page.locator('button:has-text("Register Identity")').click();

  console.log("⌛ [Step] Waiting for transaction approval...");
  await page.waitForTimeout(1000);
  await metamask.confirmTransaction();

  console.log("🗳️ [Step] Casting vote - part 2");
  await page.locator('button:has-text("Cast Vote")').click();

  console.log("⏳ [Step] Waiting for on-chain result (200s)");
  await page.waitForTimeout(240_000);

  console.log("📊 [Step] Navigating to Historic Polls");
  await page.locator('a:has-text("Historic polls")').click();

  console.log("✅ [Step] Verifying result text...");
  await expect(page.locator("h1")).toHaveText("Historic polls");
  await expect(
    page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-0'] h3"),
  ).toHaveText("100%");
  await expect(
    page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-1'] h3"),
  ).toHaveText("0%");

  console.log("🏁 [TEST COMPLETE] CRISP smoke test finished successfully");
});
