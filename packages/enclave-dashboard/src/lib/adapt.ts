// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Adapters: shape on-chain data into the prop shapes the React components expect.

import { formatUnits, numberToHex } from 'viem'
import type { HistoryEntry, Poll } from '../data'
import type { E3FullDetails, E3Summary } from './e3'
import { decodeCrispTally } from './e3'
import { formatE3Id, isCrispProgram, pollMetaFor, programName, shortAddr, shortHash } from './pollMeta'

// Fee token is MockUSDC (6 decimals) on the Sepolia deployment.
const FEE_DECIMALS = 6
function fmtUsdc(v: bigint | undefined): string {
  if (v == null) return '—'
  return `${formatUnits(v, FEE_DECIMALS)} USDC`
}

export function adaptTodaysPoll(detail: E3FullDetails): Poll {
  const meta = pollMetaFor(detail.id)
  const tally = decodeCrispTally(detail.plaintextOutput)
  const totals: Record<string, number> = {}
  meta.options.forEach((o, i) => {
    totals[o.id] = tally && tally[i] != null ? tally[i] : 0
  })
  const winnerKey =
    tally && tally.length > 0 ? (meta.options[tally.indexOf(Math.max(...tally))]?.id ?? meta.options[0].id) : meta.options[0].id

  const openedTs = detail.inputWindow[0]
  const closesTs = detail.inputWindow[1]

  return {
    id: formatE3Id(detail.id),
    question: meta.question,
    context: meta.context,
    options: meta.options,
    opened: openedTs > 0n ? fmtUtc(openedTs) : '—',
    closes: closesTs > 0n ? fmtUtc(closesTs) : '—',
    closesTs: Number(closesTs),
    ballotCount: detail.ballotCount,
    result: { winner: winnerKey, totals },
  }
}

export function adaptHistoryEntries(list: E3Summary[], detailsCache: Map<string, E3FullDetails>): HistoryEntry[] {
  return list.map((s) => {
    const detail = detailsCache.get(s.id.toString())
    const meta = pollMetaFor(s.id)
    let resultText = 'Pending'
    if (detail) {
      const tally = decodeCrispTally(detail.plaintextOutput)
      if (tally && tally.length > 0) {
        const total = tally.reduce((a, b) => a + b, 0)
        const max = Math.max(...tally)
        const pct = total > 0 ? Math.round((max / total) * 100) : 0
        const winnerIdx = tally.indexOf(max)
        const winnerLabel = meta.options[winnerIdx]?.label ?? 'Outcome'
        const verdict = /^no/i.test(winnerLabel) ? 'Declined' : /^abs/i.test(winnerLabel) ? 'Inconclusive' : 'Approved'
        resultText = `${verdict} · ${pct}%`
      }
    }
    return {
      id: formatE3Id(s.id),
      question: meta.question,
      closed: s.inputWindow[1] > 0n ? fmtDate(s.inputWindow[1]) : 'in progress',
      duration: s.inputWindow[1] > s.inputWindow[0] && s.inputWindow[0] > 0n ? fmtDuration(s.inputWindow[1] - s.inputWindow[0]) : '—',
      ballotCount: detail?.ballotCount ?? 0,
      result: resultText,
    }
  })
}

export function adaptInspectorE3List(list: E3Summary[]) {
  return list.map((s) => ({
    id: formatE3Id(s.id),
    label: isCrispProgram(s.e3Program) ? pollMetaFor(s.id).question.slice(0, 64) : programName(s.e3Program),
  }))
}

// ─── Inspector detail shape (consumed by Inspector.tsx) ──────────────────────

export type InspectorEvent = {
  t: string
  block: number | string
  name: string
  stage: string
  tx: string
  gas: string
}

export type InspectorDetail = {
  id: string
  program: string
  programAddr: string
  requestedBy: string
  requestedByLabel: string
  requestedTx: string
  requestedAt: string
  requestedBlock: number
  currentStage: number
  summary: string
  committee: { size: number; threshold: number; selectionSeed: string; drawnAt: string; note: string }
  fees: {
    feeEscrowed: string
    committeeReward: string
    currency: string
  }
  keygen: {
    scheme: string
    finalizedAt: string
    publishedAt: string
    publishedTx: string
    publicKey: string
  }
  input: {
    openedAt: string
    closesAt: string
    inputsReceived: string
    firstBallotAt: string
    lastBallotAt: string
  }
  compute: { status: string; note: string }
  decryption: { status: string; note: string; threshold: number; committeeSize: number }
  publication: { status: string; note: string }
  events: InspectorEvent[]
}

const ZERO_HASH = '0x' + '0'.repeat(64)

export function adaptInspectorDetail(detail: E3FullDetails | null): InspectorDetail | null {
  if (!detail) return null
  const isCrisp = isCrispProgram(detail.e3Program)
  const meta = pollMetaFor(detail.id)
  const inputsReceived = detail.inputsTracked ? detail.ballotCount.toLocaleString() : '—'
  const sharesRequired = detail.committeeThreshold[0] || 0
  const committeeSize = detail.committeeThreshold[1] || detail.committeeMembers.length

  return {
    id: formatE3Id(detail.id),
    program: programName(detail.e3Program),
    programAddr: shortAddr(detail.e3Program),
    requestedBy: shortAddr(detail.requester),
    requestedByLabel: 'Requester',
    requestedTx: detail.requestTxHash,
    requestedAt: detail.requestedAt ? fmtUtcFromUnix(detail.requestedAt) : `block #${detail.requestBlock.toString()}`,
    requestedBlock: Number(detail.requestBlock),
    currentStage: detail.uiStageIdx,
    summary: isCrisp ? meta.question : `Encrypted execution ${formatE3Id(detail.id)}`,

    committee: {
      size: committeeSize,
      threshold: sharesRequired,
      selectionSeed: detail.seed > 0n ? shortHash(numberToHex(detail.seed, { size: 32 })) : '—',
      drawnAt: detail.committeeFinalizedAt ? fmtUtcFromUnix(detail.committeeFinalizedAt) : '—',
      note: 'Identities are sealed. Only the count and threshold are public.',
    },

    fees: {
      feeEscrowed: fmtUsdc(detail.feeEscrowed),
      committeeReward: fmtUsdc(detail.committeeReward),
      currency: 'USDC · Sepolia',
    },

    keygen: {
      scheme: detail.encryptionSchemeId && detail.encryptionSchemeId !== ZERO_HASH ? shortHash(detail.encryptionSchemeId) : '—',
      finalizedAt: detail.committeeFinalizedAt ? fmtUtcFromUnix(detail.committeeFinalizedAt) : '—',
      publishedAt: detail.committeePublishedAt ? fmtUtcFromUnix(detail.committeePublishedAt) : '—',
      publishedTx: detail.committeePublishedTx ?? '—',
      publicKey: detail.committeePublicKey && detail.committeePublicKey !== ZERO_HASH ? shortHash(detail.committeePublicKey) : '—',
    },

    input: {
      openedAt: detail.inputWindow[0] > 0n ? fmtUtc(detail.inputWindow[0]) : '—',
      closesAt: detail.inputWindow[1] > 0n ? fmtUtc(detail.inputWindow[1]) : '—',
      inputsReceived,
      firstBallotAt: ballotTime(detail.ballotEvents[0]),
      lastBallotAt: ballotTime(detail.ballotEvents[detail.ballotEvents.length - 1]),
    },

    compute: {
      status: detail.uiStageIdx >= 4 ? 'active' : 'pending',
      note:
        detail.uiStageIdx < 4
          ? 'Compute begins automatically when the input window closes.'
          : "The program's FHE computation runs over the encrypted inputs, without decrypting any individual input.",
    },

    decryption: {
      status: detail.uiStageIdx >= 5 ? 'active' : 'pending',
      note:
        sharesRequired > 0
          ? `A threshold of ${sharesRequired} of ${committeeSize} committee members must each publish a partial decryption to recover the result.`
          : 'Threshold decryption begins after compute.',
      threshold: sharesRequired,
      committeeSize,
    },

    publication: {
      status: detail.resultTxHash ? 'complete' : 'pending',
      note: detail.resultTxHash
        ? `Result published in tx ${shortHash(detail.resultTxHash)}.`
        : 'Final result will be written on-chain. Individual ballots remain encrypted.',
    },

    events: buildEventLog(detail),
  }
}

function ballotTime(b?: { blockNumber: bigint; timestamp?: number }): string {
  if (!b) return '—'
  return b.timestamp ? fmtClock(b.timestamp) : `block #${b.blockNumber.toString()}`
}

function buildEventLog(d: E3FullDetails): InspectorEvent[] {
  const evs: InspectorEvent[] = []
  evs.push({
    t: d.requestedAt ? fmtClock(d.requestedAt) : '—',
    block: Number(d.requestBlock),
    name: 'E3Requested',
    stage: 'Requested',
    tx: shortHash(d.requestTxHash),
    gas: '—',
  })
  if (d.committeeFinalizedTx) {
    evs.push({
      t: d.committeeFinalizedAt ? fmtClock(d.committeeFinalizedAt) : '—',
      block: d.committeeFinalizedBlock != null ? Number(d.committeeFinalizedBlock) : '—',
      name: 'CommitteeFinalized',
      stage: 'Committee Selected',
      tx: shortHash(d.committeeFinalizedTx),
      gas: '—',
    })
  }
  if (d.committeePublishedTx) {
    evs.push({
      t: d.committeePublishedAt ? fmtClock(d.committeePublishedAt) : '—',
      block: d.committeePublishedBlock != null ? Number(d.committeePublishedBlock) : '—',
      name: 'CommitteePublished',
      stage: 'Keygen',
      tx: shortHash(d.committeePublishedTx),
      gas: '—',
    })
  }
  d.ballotEvents.slice(0, 5).forEach((b) => {
    evs.push({
      t: b.timestamp ? fmtClock(b.timestamp) : '—',
      block: Number(b.blockNumber),
      name: 'InputPublished',
      stage: 'Input Window',
      tx: shortHash(b.txHash),
      gas: '—',
    })
  })
  if (d.ballotEvents.length > 5) {
    evs.push({
      t: '—',
      block: '—',
      name: `InputPublished (×${d.ballotEvents.length - 5} more)`,
      stage: 'Input Window',
      tx: '—',
      gas: '—',
    })
  }
  if (d.resultTxHash) {
    evs.push({
      t: d.resultAt ? fmtClock(d.resultAt) : '—',
      block: d.resultBlock != null ? Number(d.resultBlock) : '—',
      name: 'PlaintextOutputPublished',
      stage: 'Published',
      tx: shortHash(d.resultTxHash),
      gas: '—',
    })
  }
  return evs
}

// ─── Time formatting ─────────────────────────────────────────────────────────

function fmtUtcFromUnix(sec: number): string {
  return fmtUtc(BigInt(sec))
}

function fmtUtc(ts: bigint): string {
  const d = new Date(Number(ts) * 1000)
  return (
    d.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric', timeZone: 'UTC' }) +
    ' · ' +
    d.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', timeZone: 'UTC', hour12: false }) +
    ' UTC'
  )
}

function fmtClock(sec: number): string {
  return (
    new Date(sec * 1000).toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZone: 'UTC',
      hour12: false,
    }) + ' UTC'
  )
}

function fmtDate(ts: bigint): string {
  return new Date(Number(ts) * 1000).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric', timeZone: 'UTC' })
}

function fmtDuration(seconds: bigint): string {
  const s = Number(seconds)
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.round(s / 60)} min`
  if (s < 86400) return `${Math.round(s / 3600)}h`
  return `${Math.round(s / 86400)} days`
}
