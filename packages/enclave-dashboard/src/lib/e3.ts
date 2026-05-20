// On-chain E3 fetchers — read events + view functions and assemble dashboard records.

import { CONTRACTS, DEPLOY_BLOCK, E3Stage, ciphernodeRegistryAbi, enclaveAbi, publicClient } from './chain'

// Helper: pull a single named event ABI item out of the typechain bundle.
function eventAbi(abi: readonly any[], name: string): any {
  const item = abi.find((x) => x.type === 'event' && x.name === name)
  if (!item) throw new Error(`event ${name} not in ABI`)
  return item
}

const ENCLAVE_E3_REQUESTED = eventAbi(enclaveAbi as any, 'E3Requested')
const ENCLAVE_INPUT_PUBLISHED = eventAbi(enclaveAbi as any, 'InputPublished')
const ENCLAVE_PLAINTEXT_PUBLISHED = eventAbi(enclaveAbi as any, 'PlaintextOutputPublished')
const REGISTRY_COMMITTEE_REQUESTED = eventAbi(ciphernodeRegistryAbi as any, 'CommitteeRequested')
const REGISTRY_COMMITTEE_FINALIZED = eventAbi(ciphernodeRegistryAbi as any, 'CommitteeFinalized')
const REGISTRY_COMMITTEE_PUBLISHED = eventAbi(ciphernodeRegistryAbi as any, 'CommitteePublished')

// Public RPCs cap getLogs range. 9_500 keeps us safely under common 10k limits.
const LOG_CHUNK = 9_500n

// An E3 is a CRISP poll only if its program contract is the CRISPProgram.
// Other E3s on the same Enclave deployment run different programs and must not
// be presented as polls.
export function isCrispE3(e3Program: string): boolean {
  return e3Program.toLowerCase() === CONTRACTS.CRISPProgram.toLowerCase()
}

// Solidity E3Stage → 7-stage UI index used by STAGES in data.ts.
// UI stages: 0 Requested → 1 Committee Selected → 2 Keygen → 3 Input Window
//          → 4 Compute → 5 Decryption → 6 Published
export function solidityStageToUiIdx(stage: number, inputWindow: [bigint, bigint]): number {
  const now = BigInt(Math.floor(Date.now() / 1000))
  switch (stage) {
    case E3Stage.None:
    case E3Stage.Requested:
      return 0 // request placed, awaiting committee
    case E3Stage.CommitteeFinalized:
      return 2 // sortition done, DKG running
    case E3Stage.KeyPublished: {
      // Key is out — voting open until window close, then compute pending.
      if (inputWindow[1] !== 0n && now >= inputWindow[1]) return 4
      return 3
    }
    case E3Stage.CiphertextReady:
      return 5 // ciphertext produced, threshold decrypting
    case E3Stage.Complete:
      return 6
    case E3Stage.Failed:
      return 6 // distinct visual state handled separately
    default:
      return 0
  }
}

export type E3Summary = {
  id: bigint
  e3Program: `0x${string}`
  requester: `0x${string}`
  requestBlock: bigint
  requestTxHash: `0x${string}`
  inputWindow: [bigint, bigint]
  committeeSize: number
  stage: number // raw Solidity E3Stage enum
}

// Whether an E3 is a "real" poll worth surfacing. Hides:
//  - Failed E3s.
//  - Abandoned E3s: voting window has closed but the E3 never reached the
//    compute phase (CiphertextReady) and never completed — i.e. it stalled
//    before producing anything. (A poll legitimately in compute/decryption has
//    a closed window but stage >= CiphertextReady, so it stays visible.)
export function isRealPoll(stage: number, inputWindowClose: bigint): boolean {
  if (stage === E3Stage.Failed) return false
  if (stage === E3Stage.Complete) return true
  const now = BigInt(Math.floor(Date.now() / 1000))
  const windowClosed = inputWindowClose !== 0n && now > inputWindowClose
  if (windowClosed && stage < E3Stage.CiphertextReady) return false
  return true
}

export type E3FullDetails = E3Summary & {
  stage: number // raw Solidity enum
  uiStageIdx: number
  committeePublicKey: `0x${string}`
  ciphertextOutput: `0x${string}`
  plaintextOutput: `0x${string}`
  // From CiphernodeRegistry:
  committeeThreshold: [number, number] // [M, N]
  committeeMembers: `0x${string}`[]
  committeeFinalizedTx?: `0x${string}`
  committeePublishedTx?: `0x${string}`
  // Aggregated:
  ballotCount: number
  ballotEvents: Array<{
    blockNumber: bigint
    txHash: `0x${string}`
    index: bigint
  }>
  resultTxHash?: `0x${string}`
}

async function getLogsChunked<T>(
  args: Omit<Parameters<typeof publicClient.getLogs>[0], 'fromBlock' | 'toBlock'>,
  from: bigint,
  to: bigint,
): Promise<T[]> {
  const out: any[] = []
  for (let start = from; start <= to; start += LOG_CHUNK + 1n) {
    const end = start + LOG_CHUNK > to ? to : start + LOG_CHUNK
    const logs = await publicClient.getLogs({ ...args, fromBlock: start, toBlock: end } as any)
    out.push(...logs)
  }
  return out as T[]
}

export async function fetchLatestBlock(): Promise<bigint> {
  return publicClient.getBlockNumber()
}

export async function fetchE3List(toBlock?: bigint): Promise<E3Summary[]> {
  const head = toBlock ?? (await fetchLatestBlock())
  const logs = await getLogsChunked<any>(
    {
      address: CONTRACTS.Enclave,
      event: ENCLAVE_E3_REQUESTED,
    },
    DEPLOY_BLOCK,
    head,
  )

  // Only CRISP-program E3s are polls. Skip everything else.
  const crispLogs = logs.filter((log) => isCrispE3(log.args.e3.e3Program))

  // Fetch the current stage of each E3 in one multicall so we can drop failed
  // and abandoned polls (see isRealPoll).
  const stages = await (publicClient.multicall as any)({
    contracts: crispLogs.map((log) => ({
      address: CONTRACTS.Enclave,
      abi: enclaveAbi,
      functionName: 'getE3Stage',
      args: [log.args.e3Id],
    })),
    allowFailure: true,
  })

  const out: E3Summary[] = crispLogs
    .map((log, i) => {
      const { e3Id, e3 } = log.args
      const stageResult = stages[i]
      const stage = stageResult.status === 'success' ? Number(stageResult.result) : E3Stage.None
      return {
        id: e3Id,
        e3Program: e3.e3Program,
        requester: e3.requester,
        requestBlock: e3.requestBlock,
        requestTxHash: log.transactionHash,
        inputWindow: [e3.inputWindow[0], e3.inputWindow[1]] as [bigint, bigint],
        committeeSize: Number(e3.committeeSize),
        stage,
      }
    })
    .filter((s) => isRealPoll(s.stage, s.inputWindow[1]))

  // Sort newest first.
  out.sort((a, b) => Number(b.requestBlock - a.requestBlock))
  return out
}

export async function fetchE3Details(e3Id: bigint, toBlock?: bigint): Promise<E3FullDetails> {
  const head = toBlock ?? (await fetchLatestBlock())

  // 1. Pull live E3 struct + stage.
  const [e3, stage] = await Promise.all([
    (publicClient.readContract as any)({
      address: CONTRACTS.Enclave,
      abi: enclaveAbi,
      functionName: 'getE3',
      args: [e3Id],
    }) as Promise<any>,
    (publicClient.readContract as any)({
      address: CONTRACTS.Enclave,
      abi: enclaveAbi,
      functionName: 'getE3Stage',
      args: [e3Id],
    }) as Promise<number>,
  ])

  // 2. Find the E3Requested tx for this id (for the inspector header).
  const requestLogs = await getLogsChunked<any>(
    {
      address: CONTRACTS.Enclave,
      event: ENCLAVE_E3_REQUESTED,
    },
    DEPLOY_BLOCK,
    head,
  )
  const requestLog = requestLogs.find((l: any) => l.args.e3Id === e3Id)
  const requestTxHash = (requestLog?.transactionHash ?? ('0x' as `0x${string}`)) as `0x${string}`

  // 3. Committee data from CiphernodeRegistry.
  const [requestedEvents, finalizedEvents, publishedEvents] = await Promise.all([
    getLogsChunked<any>(
      {
        address: CONTRACTS.CiphernodeRegistry,
        event: REGISTRY_COMMITTEE_REQUESTED,
        args: { e3Id },
      } as any,
      DEPLOY_BLOCK,
      head,
    ),
    getLogsChunked<any>(
      {
        address: CONTRACTS.CiphernodeRegistry,
        event: REGISTRY_COMMITTEE_FINALIZED,
        args: { e3Id },
      } as any,
      DEPLOY_BLOCK,
      head,
    ),
    getLogsChunked<any>(
      {
        address: CONTRACTS.CiphernodeRegistry,
        event: REGISTRY_COMMITTEE_PUBLISHED,
        args: { e3Id },
      } as any,
      DEPLOY_BLOCK,
      head,
    ),
  ])

  const reqLog = requestedEvents[0]
  const finLog = finalizedEvents[0]
  const pubLog = publishedEvents[0]

  const threshold: [number, number] = reqLog ? [Number(reqLog.args.threshold[0]), Number(reqLog.args.threshold[1])] : [0, 0]
  const members: `0x${string}`[] = (finLog?.args?.committee ?? pubLog?.args?.nodes ?? []) as `0x${string}`[]

  // 4. Ballots (InputPublished) + result.
  const [inputs, results] = await Promise.all([
    getLogsChunked<any>(
      {
        address: CONTRACTS.Enclave,
        event: ENCLAVE_INPUT_PUBLISHED,
        args: { e3Id },
      } as any,
      DEPLOY_BLOCK,
      head,
    ),
    getLogsChunked<any>(
      {
        address: CONTRACTS.Enclave,
        event: ENCLAVE_PLAINTEXT_PUBLISHED,
        args: { e3Id },
      } as any,
      DEPLOY_BLOCK,
      head,
    ),
  ])

  return {
    id: e3Id,
    e3Program: e3.e3Program,
    requester: e3.requester,
    requestBlock: e3.requestBlock,
    requestTxHash,
    inputWindow: [e3.inputWindow[0], e3.inputWindow[1]] as [bigint, bigint],
    committeeSize: Number(e3.committeeSize),
    stage,
    uiStageIdx: solidityStageToUiIdx(stage, [e3.inputWindow[0], e3.inputWindow[1]]),
    committeePublicKey: e3.committeePublicKey,
    ciphertextOutput: e3.ciphertextOutput,
    plaintextOutput: e3.plaintextOutput,
    committeeThreshold: threshold,
    committeeMembers: members,
    committeeFinalizedTx: finLog?.transactionHash,
    committeePublishedTx: pubLog?.transactionHash,
    ballotCount: inputs.length,
    ballotEvents: inputs.map((l: any) => ({
      blockNumber: l.blockNumber,
      txHash: l.transactionHash,
      index: l.args.index,
    })),
    resultTxHash: results[0]?.transactionHash,
  }
}

// Decode CRISP tally from PlaintextOutputPublished bytes. CRISPProgram packs
// it as a uint64[] (per CRISPProgram.decodeTally). We surface the raw counts
// and let the UI label them by option index.
export function decodeCrispTally(plaintextOutput: `0x${string}`): number[] | null {
  if (!plaintextOutput || plaintextOutput === '0x' || plaintextOutput.length < 4) return null
  try {
    // The data is abi-encoded `bytes` → uint64[] inside. Lightweight decoder:
    // strip 0x, treat each 32-byte word as a uint256. The first word is offset,
    // second is length, then the data. We expect uint64 values one per word.
    const hex = plaintextOutput.slice(2)
    if (hex.length < 128) return null
    const lengthWord = hex.slice(64, 128)
    const len = parseInt(lengthWord, 16)
    if (!Number.isFinite(len) || len > 1024) return null
    const out: number[] = []
    for (let i = 0; i < len; i++) {
      const word = hex.slice(128 + i * 64, 128 + (i + 1) * 64)
      if (word.length !== 64) return null
      out.push(Number(BigInt('0x' + word)))
    }
    return out
  } catch {
    return null
  }
}
