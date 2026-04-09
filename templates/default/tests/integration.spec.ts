// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import {
  EnclaveSDK,
  calculateInputWindow,
  DEFAULT_COMPUTE_PROVIDER_PARAMS,
  encodeComputeProviderParams,
  decodePlaintextOutput,
  CommitteeSize,
  ThresholdBfvParamsPresetNames,
} from '@enclave-e3/sdk'
import { EnclaveEventType, RegistryEventType } from '@enclave-e3/sdk/events'
import type { AllEventTypes, EnclaveEvent } from '@enclave-e3/sdk/events'
import { E3Stage } from '@enclave-e3/sdk/contracts'
import type { E3 } from '@enclave-e3/sdk/contracts'
import { createWalletClient, hexToBytes, http } from 'viem'
import assert from 'assert'

import { describe, expect, it } from 'vitest'
import { publishInput } from '../server/input'
import { privateKeyToAccount } from 'viem/accounts'
import { anvil } from 'viem/chains'

export function getContractAddresses() {
  return {
    enclave: process.env.ENCLAVE_ADDRESS as `0x${string}`,
    ciphernodeRegistry: process.env.REGISTRY_ADDRESS as `0x${string}`,
    bondingRegistry: process.env.BONDING_REGISTRY_ADDRESS as `0x${string}`,
    e3Program: process.env.E3_PROGRAM_ADDRESS as `0x${string}`,
    feeToken: process.env.FEE_TOKEN_ADDRESS as `0x${string}`,
  }
}

type E3Shared = {
  e3Id: bigint
  e3Program: string
  e3: E3
}

type E3StateRequested = E3Shared & {
  type: 'requested'
}

type E3StatePublished = E3Shared & {
  type: 'committee_published'
  publicKey: `0x${string}`
}

type E3StateOutputPublished = E3Shared & {
  type: 'output_published'
  plaintextOutput: string
}

type E3State = E3StateRequested | E3StatePublished | E3StateOutputPublished

async function setupEventListeners(sdk: EnclaveSDK, store: Map<bigint, E3State>) {
  async function waitForEvent<T extends AllEventTypes>(
    type: T,
    trigger?: () => Promise<void>,
    timeoutMs?: number,
  ): Promise<EnclaveEvent<T>> {
    return new Promise((resolve, reject) => {
      let settled = false
      let timer: ReturnType<typeof setTimeout> | undefined

      const handler = (event: EnclaveEvent<T>) => {
        if (settled) return
        settled = true
        if (timer !== undefined) clearTimeout(timer)
        sdk.off(type, handler)
        resolve(event)
      }

      const fail = (err: unknown) => {
        if (settled) return
        settled = true
        if (timer !== undefined) clearTimeout(timer)
        sdk.off(type, handler)
        reject(err)
      }

      // Use onEnclaveEvent so `handler` is the actual registered reference
      // (sdk.once wraps in an internal closure, making sdk.off unable to remove it)
      sdk.onEnclaveEvent(type, handler).catch(fail)

      if (timeoutMs !== undefined) {
        timer = setTimeout(() => {
          fail(new Error(`Timed out waiting for event: ${type} after ${timeoutMs}ms`))
        }, timeoutMs)
      }

      if (trigger) {
        trigger().catch(fail)
      }
    })
  }

  await sdk.onEnclaveEvent(EnclaveEventType.E3_REQUESTED, (event) => {
    const id = event.data.e3Id

    if (store.has(id)) {
      throw new Error('E3 has already been requested ')
    }

    store.set(event.data.e3Id, {
      type: 'requested',
      ...event.data,
    })
  })

  await sdk.onEnclaveEvent(RegistryEventType.COMMITTEE_PUBLISHED, (event) => {
    const id = event.data.e3Id

    const state = store.get(id)

    if (!state) {
      throw new Error(`State for ID '${id}'not found.`)
    }

    if (state.type !== 'requested') {
      throw new Error(`State must be in the requested state`)
    }

    store.set(id, {
      publicKey: event.data.publicKey as `0x${string}`,
      ...state,
      type: 'committee_published',
    })
  })

  await sdk.onEnclaveEvent(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, (event) => {
    const id = event.data.e3Id

    const state = store.get(id)

    if (!state) {
      throw new Error(`State for ID '${id}' not found.`)
    }

    if (state.type !== 'committee_published') {
      throw new Error(`State must be in the committee_published state`)
    }

    store.set(id, {
      ...state,
      plaintextOutput: event.data.plaintextOutput,
      type: 'output_published',
    })
  })

  return { waitForEvent }
}

describe('Integration', () => {
  console.log('Testing...')

  const contracts = getContractAddresses()

  const testPrivateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'

  const store = new Map<bigint, E3State>()
  const sdk = EnclaveSDK.create({
    contracts: {
      enclave: contracts.enclave,
      ciphernodeRegistry: contracts.ciphernodeRegistry,
      feeToken: contracts.feeToken,
    },
    rpcUrl: 'ws://localhost:8545',
    chain: anvil,
    thresholdBfvParamsPresetName: ThresholdBfvParamsPresetNames[1],
    privateKey: testPrivateKey,
  })

  const publicClient = sdk.getPublicClient()

  const account = privateKeyToAccount(testPrivateKey)

  const walletClient = createWalletClient({
    account,
    chain: anvil,
    transport: http('http://localhost:8545'),
  })

  it('should run an integration test', async () => {
    const { waitForEvent } = await setupEventListeners(sdk, store)

    const committeeSize = CommitteeSize.Micro
    const duration = 1000
    const inputWindow = await calculateInputWindow(publicClient, duration)
    const computeProviderParams = encodeComputeProviderParams(
      DEFAULT_COMPUTE_PROVIDER_PARAMS,
      true, // Mock the compute provider parameters, return 32 bytes of 0x00
    )

    let state
    let event

    // Verify fee quoting works
    const requestParams = {
      committeeSize,
      inputWindow,
      e3Program: contracts.e3Program,
      paramSet: 1, // ParamSet.Secure8192
      computeProviderParams,
      proofAggregationEnabled: false,
    }
    const quote = await sdk.getE3Quote(requestParams)
    console.log('E3 quote:', quote)
    assert(quote >= 0n, 'E3 quote should be a non-negative bigint')

    // Approve fee token
    console.log('Approving fee token...')
    const hash = await sdk.approveFeeToken(quote)
    console.log('Fee token approved:', hash)

    await new Promise((resolve) => setTimeout(resolve, 1000))

    // REQUEST phase
    const timeoutMs = duration * 1000

    await waitForEvent(
      EnclaveEventType.E3_REQUESTED,
      async () => {
        console.log('Requested E3...')
        await sdk.requestE3(requestParams)
      },
      timeoutMs,
    )

    state = store.get(0n)
    assert(state, 'store should have E3State but it was falsey')
    assert.strictEqual(state.e3Id, 0n)
    assert.strictEqual(state.type, 'requested')
    console.log('E3 Sucessfully Requested!')

    // Verify E3 stage after request
    const stageAfterRequest = await sdk.getE3Stage(state.e3Id)
    assert.strictEqual(stageAfterRequest, E3Stage.Requested, 'E3 stage should be Requested after requestE3')

    // Ciphernodes will publish a public key within the COMMITTEE_PUBLISHED event
    event = await waitForEvent(RegistryEventType.COMMITTEE_PUBLISHED, undefined, timeoutMs)

    const publicKeyBytes = hexToBytes(event.data.publicKey as `0x${string}`)

    state = store.get(state.e3Id)
    assert(state, 'store should have E3State but it was falsey')
    assert.strictEqual(state.type, 'committee_published')
    assert.strictEqual(state.publicKey, event.data.publicKey)

    // Verify E3 stage after committee published
    const stageAfterCommittee = await sdk.getE3Stage(state.e3Id)
    assert.strictEqual(stageAfterCommittee, E3Stage.KeyPublished, 'E3 stage should be KeyPublished after committee published')

    // INPUT PUBLISHING phase
    console.log('PUBLISHING PRIVATE INPUT')
    const num1 = 1n
    const num2 = 2n

    console.log('ENCRYPTING NUMBERS')
    const enc1 = await sdk.encryptNumber(num1, publicKeyBytes)
    const enc2 = await sdk.encryptNumber(num2, publicKeyBytes)

    let txHash = await publishInput(
      walletClient,
      state.e3Id,
      `0x${Array.from(enc1, (b) => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`,
      account.address,
      contracts.e3Program,
    )
    await sdk.waitForTransaction(txHash)
    txHash = await publishInput(
      walletClient,
      state.e3Id,
      `0x${Array.from(enc2, (b) => b.toString(16).padStart(2, '0')).join('')}` as `0x${string}`,
      account.address,
      contracts.e3Program,
    )
    await sdk.waitForTransaction(txHash)

    const plaintextEvent = await waitForEvent(EnclaveEventType.PLAINTEXT_OUTPUT_PUBLISHED, undefined, timeoutMs)

    const result = decodePlaintextOutput(plaintextEvent.data.plaintextOutput)
    assert(result !== null, 'Failed to decode plaintext output')

    expect(BigInt(result)).toBe(num1 + num2)
    console.log('Answer was correct')
  }, 9999999)
})
