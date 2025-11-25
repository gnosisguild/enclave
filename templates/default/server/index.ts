// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import express, { Request, Response } from 'express'
import { EnclaveSDK, EnclaveEventType, type E3ActivatedData, FheProtocol } from '@enclave-e3/sdk'
import { Log, PublicClient } from 'viem'
import { handleTestInteraction } from './testHandler'
import { getCheckedEnvVars } from './utils'
import { callFheRunner } from './runner'
import { ProgramEventType, RawInputPublishedEvent } from './types'
import { MyProgram__factory } from '../types/factories/contracts'

interface E3Session {
  e3Id: bigint
  expiration: bigint
  e3ProgramParams?: string
  inputs: Array<{ data: string; index: bigint }>
  isProcessing: boolean
  isCompleted: boolean
}

const e3Sessions = new Map<string, E3Session>()

async function createPrivateSDK() {
  const { CHAIN_ID, PRIVATE_KEY, CIPHERNODE_REGISTRY_CONTRACT, ENCLAVE_CONTRACT, FEE_TOKEN_CONTRACT, RPC_URL } = getCheckedEnvVars()

  if (!isSupportedChain(CHAIN_ID)) {
    throw new Error(`Unsupported CHAIN_ID: ${CHAIN_ID}`)
  }

  const sdk = EnclaveSDK.create({
    rpcUrl: RPC_URL,
    privateKey: PRIVATE_KEY as `0x${string}`,
    contracts: {
      enclave: ENCLAVE_CONTRACT as `0x${string}`,
      ciphernodeRegistry: CIPHERNODE_REGISTRY_CONTRACT as `0x${string}`,
      feeToken: FEE_TOKEN_CONTRACT as `0x${string}`,
    },
    chainId: CHAIN_ID,
    protocol: FheProtocol.BFV,
  })

  await sdk.initialize()
  return sdk
}

async function runProgram(e3Id: bigint): Promise<void> {
  const sessionKey = e3Id.toString()
  const session = e3Sessions.get(sessionKey)

  if (!session || session.isProcessing || session.isCompleted) {
    return
  }

  console.log(`📊 Processing E3 session ${e3Id} with ${session.inputs.length} inputs`)

  try {
    session.isProcessing = true

    if (session.inputs.length <= 1) {
      console.log(`⏭️  Skipping E3 ${e3Id}: not enough inputs (${session.inputs.length})`)
      session.isCompleted = true
      return
    }

    let e3ProgramParams = session.e3ProgramParams
    if (!e3ProgramParams) {
      const sdk = await createPrivateSDK()
      const e3Details = (await sdk.getE3(e3Id)) as any
      e3ProgramParams = e3Details.e3ProgramParams
      session.e3ProgramParams = e3ProgramParams
    }

    const ciphertextInputs: Array<[string, number]> = session.inputs.map((input) => [input.data, Number(input.index)])

    console.log(`🔄 Calling FHE runner for E3 ${e3Id}...`)
    await callFheRunner(e3Id, e3ProgramParams!, ciphertextInputs)

    console.log(`✅ E3 ${e3Id} sent to FHE runner - awaiting callback`)
  } catch (error) {
    console.error(`❌ Error processing E3 ${e3Id}:`, error)
    session.isProcessing = false
  }
}

function defer() {
  let resolve: () => void = () => {}
  let reject: (e?: any) => void = () => {}

  const promise = new Promise<void>((res, rej) => {
    resolve = res
    reject = rej
  })

  return {
    promise,
    resolve,
    reject,
  }
}

type Defer = ReturnType<typeof defer>

const currentlyActivating = new Map<bigint, Defer>()

function getActivationDefer(e3Id: bigint): Defer {
  let d = currentlyActivating.get(e3Id)
  if (!d) {
    const def = defer()
    currentlyActivating.set(e3Id, def)
    return def
  }
  return d
}

async function handleE3ActivatedEvent(event: any) {
  const data = event.data as E3ActivatedData
  const e3Id = data.e3Id
  const expiration = data.expiration

  // This allows us to wait until the session has been activated avoiding race conditions
  const def = getActivationDefer(e3Id)

  console.log(`🎯 E3 Activated: ${e3Id}, expiration: ${expiration}`)

  const sessionKey = e3Id.toString()

  if (!e3Sessions.has(sessionKey)) {
    const sdk = await createPrivateSDK()
    console.log('📡 Fetching E3 data from contract...')

    const e3 = await sdk.getE3(e3Id)
    console.log('✅ Received E3 data from contract.')

    e3Sessions.set(sessionKey, {
      e3Id,
      e3ProgramParams: e3.e3ProgramParams,
      expiration,
      inputs: [],
      isProcessing: false,
      isCompleted: false,
    })
    def.resolve()
  }

  const currentTime = BigInt(Math.floor(Date.now() / 1000))
  const sleepSeconds = expiration > currentTime ? Number(expiration - currentTime) : 0

  if (sleepSeconds > 0) {
    console.log(`⏰ Scheduling E3 ${e3Id} processing in ${sleepSeconds} seconds...`)
    setTimeout(async () => {
      await runProgram(e3Id)
    }, sleepSeconds * 1000)
  } else {
    console.log(`⚡ E3 ${e3Id} already expired, processing immediately...`)
    await runProgram(e3Id)
  }
}

async function handleInputPublishedEvent(event: RawInputPublishedEvent) {
  const e3Id = event.args.e3Id

  console.log(`📝 Input Published for E3 ${e3Id}: index ${event.args.index}`)

  const sessionKey = e3Id.toString()

  // Ensure the session is available
  await getActivationDefer(e3Id).promise

  const session = e3Sessions.get(sessionKey)

  if (session) {
    session.inputs.push({
      data: event.args.data,
      index: event.args.index,
    })
    console.log(`📊 E3 ${e3Id} now has ${session.inputs.length} inputs`)
  } else {
    console.warn(`⚠️  Received input for unknown E3 session: ${e3Id}`)
  }
}

async function listenToInputPublishedEvents(publicClient: PublicClient, address: `0x${string}`, fromBlock: bigint) {
  publicClient.watchContractEvent({
    address,
    abi: MyProgram__factory.abi,
    eventName: ProgramEventType.INPUT_PUBLISHED,
    fromBlock,
    async onLogs(logs: Log[]) {
      for (let i = 0; i < logs.length; i++) {
        const log = logs[i]
        if (!log) {
          console.log('warning: Log was falsy when a log was expected!')
          break
        }
        const eventData = log as unknown as RawInputPublishedEvent
        await handleInputPublishedEvent(eventData)
      }
    },
  })
}

async function setupEventListeners() {
  const sdk = await createPrivateSDK()

  const { E3_PROGRAM_ADDRESS: PROGRAM_ADDRESS } = getCheckedEnvVars()

  console.log('📡 Setting up event listeners...')

  sdk.onEnclaveEvent(EnclaveEventType.E3_ACTIVATED, handleE3ActivatedEvent)
  await listenToInputPublishedEvents(sdk.getPublicClient(), PROGRAM_ADDRESS as `0x${string}`, 0n)

  console.log('✅ Event listeners set up successfully')
}

function isValidHexString(value: string): value is `0x${string}` {
  return value.startsWith('0x') && /^0x[a-fA-F0-9]*$/.test(value)
}

function isSupportedChain(value: any): value is keyof typeof EnclaveSDK.chains {
  return value in EnclaveSDK.chains
}

async function handleWebhookRequest(req: Request, res: Response) {
  try {
    console.log('📨 Webhook received:')

    const { e3_id, ciphertext, proof } = req.body
    if (e3_id === undefined || !ciphertext || !proof) {
      console.error('Missing required fields: e3_id, ciphertext, proof')

      res.status(400).json({ error: 'Missing required fields: e3_id, ciphertext, proof' })
      return
    }

    if (!isValidHexString(ciphertext) || !isValidHexString(proof)) {
      console.error('ciphertext and proof must be valid hex strings')
      res.status(400).json({ error: 'ciphertext and proof must be valid hex strings' })
      return
    }

    console.log(`🔄 Publishing output for E3 ${e3_id}...`)

    const sdk = await createPrivateSDK()
    await sdk.publishCiphertextOutput(BigInt(e3_id), ciphertext, proof)

    // Mark session as completed
    const sessionKey = e3_id.toString()
    const session = e3Sessions.get(sessionKey)
    if (session) {
      session.isCompleted = true
      session.isProcessing = false
      console.log(`✅ Successfully completed E3 ${e3_id}`)
    }

    res.json({ status: 'success', e3_id })
  } catch (error) {
    console.error('❌ Webhook processing failed:', error)
    res.status(500).json({ error: 'Internal server error' })
  }
}

function handleGetSessions(req: Request, res: Response) {
  const sessions = Array.from(e3Sessions.entries()).map(([key, session]) => ({
    e3Id: key,
    expiration: session.expiration.toString(),
    inputCount: session.inputs.length,
    isProcessing: session.isProcessing,
    isCompleted: session.isCompleted,
  }))
  res.json(sessions)
}

const app = express()
app.use(express.json())

app.post('/', handleWebhookRequest)
app.get('/sessions', handleGetSessions)

// This allows us to test interaction between server and program
// TEST_MODE=1 pnpm dev:server
if (process.env.TEST_MODE) {
  app.get('/test', handleTestInteraction)
}

app.get('/sessions', handleGetSessions)

async function startServer() {
  try {
    await setupEventListeners()

    const PORT = process.env.PORT ? parseInt(process.env.PORT) : 8080
    app.listen(PORT, '0.0.0.0', () => {
      console.log(`🚀 Enclave Server listening on port ${PORT}`)
      console.log(`📡 Event listeners active`)
      console.log(`📊 Sessions: http://localhost:${PORT}/sessions`)
    })
  } catch (error) {
    console.error('❌ Failed to start server:', error)
    process.exit(1)
  }
}

startServer().catch(console.error)
