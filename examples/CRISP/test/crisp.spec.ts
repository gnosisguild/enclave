// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ConsoleMessage, Page } from '@playwright/test'
import { testWithSynpress } from '@synthetixio/synpress'
import { MetaMask, metaMaskFixtures } from '@synthetixio/synpress/playwright'
import basicSetup from './wallet-setup/basic.setup'
import { execSync } from 'child_process'
import { config } from 'dotenv'
import path from 'path'

config({ path: path.join(process.cwd(), 'server', '.env') })

async function runCliInit(): Promise<number> {
  try {
    // Execute the command and wait for it to complete
    const output = execSync('pnpm cli init --token-address 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 --balance-threshold 1000', {
      encoding: 'utf-8',
    })
    console.log('Command output:', output)
    const lines = output.trim().split('\n')
    const lastLine = lines[lines.length - 1].trim()
    const e3Id = parseInt(lastLine, 10)
    if (isNaN(e3Id)) {
      throw new Error(`Failed to parse e3Id from CLI output: ${lastLine}`)
    }
    return e3Id
  } catch (error) {
    console.error('Error executing command:', error)
    throw error
  }
}

async function checkE3Ready(e3id: number): Promise<boolean> {
  try {
    const output = execSync(`pnpm cli check-e3-ready --e3id ${e3id}`, {
      encoding: 'utf-8',
    })
    const lines = output.trim().split('\n')
    const lastLine = lines[lines.length - 1].trim()
    return lastLine === 'true'
  } catch (error) {
    console.error('Error checking e3 activation:', error)
    return false
  }
}

async function waitForE3Ready(e3id: number, maxWaitMs: number = 80000): Promise<void> {
  const startTime = Date.now()
  while (Date.now() - startTime < maxWaitMs) {
    const isActivated = await checkE3Ready(e3id)
    if (isActivated) {
      console.log(`E3 ${e3id} is ready`)
      return
    }
    await new Promise((resolve) => setTimeout(resolve, 2000))
  }
  throw new Error(`E3 ${e3id} was not ready within ${maxWaitMs}ms`)
}

const test = testWithSynpress(metaMaskFixtures(basicSetup))
const { expect } = test

async function ensureHomePageLoaded(page: Page) {
  return await expect(page.locator('h4')).toHaveText('Coercion-Resistant Impartial Selection Protocol')
}

function log(msg: string) {
  console.log(`[playwright] ${msg}`)
}

// ConnectKit modal animations + app initialization (initialLoad/switchChain)
// can cause the MetaMask button to be detached from the DOM or the page to
// navigate while the modal is opening. Retry the whole flow up to 3 times.
async function connectWalletWithRetry(page: Page, maxAttempts = 3) {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      await page.waitForLoadState('load')

      const connectWalletBtn = page.locator('button:has-text("Connect Wallet")')
      const metamaskBtn = page.locator('button:has-text("MetaMask")')

      // Only open the modal if MetaMask option isn't already visible
      if (!(await metamaskBtn.isVisible().catch(() => false))) {
        log(`clicking Connect Wallet (attempt ${attempt})...`)
        await connectWalletBtn.click({ timeout: 10_000 })
      }

      log(`clicking MetaMask (attempt ${attempt})...`)
      await metamaskBtn.click({ timeout: 15_000 })
      return
    } catch (error) {
      if (attempt === maxAttempts) throw error
      log(`wallet connect attempt ${attempt} failed, retrying...`)
      // Dismiss any open modal before retrying
      await page.keyboard.press('Escape').catch(() => {})
      await page.waitForTimeout(2_000)
    }
  }
}

test('CRISP smoke test', async ({ context, page, metamaskPage, extensionId }) => {
  page.on('console', (msg: ConsoleMessage) => {
    console.log(msg.text())
  })

  log('============================================')
  log('      STARTING YOUR PLAYWRIGHT TEST!        ')
  log('============================================')

  log('Creating new Metamask...')
  const metamask = new MetaMask(context, metamaskPage, basicSetup.walletPassword, extensionId)

  log('runCliInit()...')
  const e3id = await runCliInit()
  log(`Got e3 id: ${e3id}`)

  await page.goto('/', { waitUntil: 'domcontentloaded' })
  await page.waitForLoadState('load')

  log(`ensureHomePageLoaded...`)
  await ensureHomePageLoaded(page)

  log(`connecting wallet via ConnectKit...`)
  await connectWalletWithRetry(page)
  log(`connecting to dapp...`)
  await metamask.connectToDapp()
  log(`clicking try demo...`)
  await page.locator('button:has-text("Try Demo")').click()

  log(`waiting for E3 Committee being published...`)
  await waitForE3Ready(e3id)
  log(`forcing page reload...`)
  await page.reload()

  log(`clicking first vote card...`)
  await page.locator("[data-test-id='poll-button-0'] > [data-test-id='card']").click()
  log(`clicking Cast Vote...`)
  await page.locator('button:has-text("Cast Vote")').click()
  log(`confirming MetaMask signature request...`)
  await metamask.confirmSignature()
  const WAIT = parseInt(process.env.E3_DURATION as string, 10) * 1000 + 45_000 // A small buffer for decryption
  log(`waiting ${WAIT}ms...`)
  await page.waitForTimeout(WAIT)
  log(`clicking historic polls button...`)
  await page.locator('a:has-text("Historic polls")').click()
  log(`asserting that Historic polls exists...`)
  await expect(page.locator('h1')).toHaveText('Historic polls')
  log(`asserting that result has 100% on the vote we clicked on...`)
  await expect(page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-0'] h3")).toHaveText('100%')
  log(`asserting that result has 0% on the vote we did not click on...`)
  await expect(page.locator("[data-test-id='poll-0-0'] [data-test-id='poll-result-1'] h3")).toHaveText('0%')

  log('============================================')
  log('        PLAYWRIGHT TEST IS COMPLETE         ')
  log('============================================')
})
