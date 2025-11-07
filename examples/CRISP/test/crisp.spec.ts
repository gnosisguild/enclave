// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { Page } from "@playwright/test";
import { testWithSynpress } from "@synthetixio/synpress";
import { MetaMask, metaMaskFixtures } from "@synthetixio/synpress/playwright";
import basicSetup from "./wallet-setup/basic.setup";
import { execSync } from "child_process";

async function runCliInit(): Promise<number> {
  try {
    // Execute the command and wait for it to complete
    const output = execSync(
      "pnpm cli init --token-address 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 --balance-threshold 1000",
      { encoding: "utf-8" },
    );
    console.log("Command output:", output);
    const lines = output.trim().split("\n");
    const lastLine = lines[lines.length - 1].trim();
    const e3Id = parseInt(lastLine, 10);
    if (isNaN(e3Id)) {
      throw new Error(`Failed to parse e3Id from CLI output: ${lastLine}`);
    }
    return e3Id;
  } catch (error) {
    console.error("Error executing command:", error);
    throw error;
  }
}

async function checkE3Activated(e3id: number): Promise<boolean> {
  try {
    const output = execSync(`pnpm cli check-activate --e3id ${e3id}`, {
      encoding: "utf-8",
    });
    const lines = output.trim().split("\n");
    const lastLine = lines[lines.length - 1].trim();
    return lastLine === "true";
  } catch (error) {
    console.error("Error checking e3 activation:", error);
    return false;
  }
}

async function waitForE3Activation(
  e3id: number,
  maxWaitMs: number = 300000,
): Promise<void> {
  const startTime = Date.now();
  while (Date.now() - startTime < maxWaitMs) {
    const isActivated = await checkE3Activated(e3id);
    if (isActivated) {
      console.log(`E3 ${e3id} is activated`);
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 2000));
  }
  throw new Error(`E3 ${e3id} was not activated within ${maxWaitMs}ms`);
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

  const e3id = await runCliInit();
  console.log(`Got e3 id: ${e3id}`);

  await page.goto("/");

  page.on("console", (...msg: any[]) => {
    console.log(...msg);
  });

  await ensureHomePageLoaded(page);
  await page.locator('button:has-text("Connect Wallet")').click();
  await page.locator('button:has-text("MetaMask")').click();
  await metamask.connectToDapp();
  await page.locator('button:has-text("Try Demo")').click();

  await waitForE3Activation(e3id);
  await page.reload();

  await page
    .locator("[data-test-id='poll-button-0'] > [data-test-id='card']")
    .click();
  await page.locator('button:has-text("Cast Vote")').click();
  await page.waitForTimeout(220_000);
  await page.locator('a:has-text("Historic polls")').click();
  await expect(page.locator("h1")).toHaveText("Historic polls");
  await expect(
    page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-0'] h3"),
  ).toHaveText("100%");
  await expect(
    page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-1'] h3"),
  ).toHaveText("0%");
});
