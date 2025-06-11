import { Page } from "@playwright/test";
import { testWithSynpress } from "@synthetixio/synpress";
import { MetaMask, metaMaskFixtures } from "@synthetixio/synpress/playwright";
import basicSetup from "./wallet-setup/basic.setup";
import { execSync } from "child_process";

async function runCliInit() {
  try {
    // Execute the command and wait for it to complete
    const output = execSync("pnpm cli init", { encoding: "utf-8" });
    console.log("Command output:", output);
    return output;
  } catch (error) {
    console.error("Error executing command:", error);
    throw error;
  }
}

const test = testWithSynpress(metaMaskFixtures(basicSetup));
const { expect } = test;

async function ensureHomePageLoaded(page: Page) {
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
  const metamask = new MetaMask(
    context,
    metamaskPage,
    basicSetup.walletPassword,
    extensionId,
  );

  await runCliInit();
  await page.goto("/");
  await ensureHomePageLoaded(page);
  await page.locator('button:has-text("Connect Wallet")').click();
  await page.locator('button:has-text("MetaMask")').click();
  await metamask.connectToDapp();
  await page.locator('button:has-text("Try Demo")').click();
  await page
    .locator("[data-test-id='poll-button-0'] > [data-test-id='card']")
    .click();
  await page.locator('button:has-text("Cast Vote")').click();
  await page.locator('button:has-text("Register Identity")').click();
  await page.waitForTimeout(1000);
  await metamask.confirmTransaction();
  await page.locator('button:has-text("Cast Vote")').click();
  await page.waitForTimeout(300000);
  await page.locator('a:has-text("Historic polls")').click();
  await expect(page.locator("h1")).toHaveText("Historic polls");
  await expect(
    page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-0'] h3"),
  ).toHaveText("100%");
  await expect(
    page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-1'] h3"),
  ).toHaveText("0%");
});
