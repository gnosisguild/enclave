// Adapters: shape on-chain data into the prop shapes the React components expect.

import { TODAYS_POLL, HISTORY, E3_DETAILS } from '../data'
import type { E3FullDetails, E3Summary } from './e3'
import { decodeCrispTally } from './e3'
import { formatE3Id, pollMetaFor, shortAddr, shortHash } from './pollMeta'

export function adaptTodaysPoll(detail: E3FullDetails | null): typeof TODAYS_POLL {
  if (!detail) return TODAYS_POLL
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
    ballotCount: detail.ballotCount,
    result: { winner: winnerKey, totals },
  }
}

export function adaptHistoryEntries(list: E3Summary[], detailsCache: Map<string, E3FullDetails>): typeof HISTORY {
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
    label: pollMetaFor(s.id).question.slice(0, 64),
  }))
}

export function adaptInspectorDetail(detail: E3FullDetails | null) {
  if (!detail) return null
  const meta = pollMetaFor(detail.id)
  const friendlyId = formatE3Id(detail.id)
  const ballotsReceived = detail.ballotCount
  const sharesRequired = detail.committeeThreshold[0] || 0
  const committeeSize = detail.committeeThreshold[1] || detail.committeeMembers.length

  return {
    id: friendlyId,
    program: meta.programLabel ?? 'CRISP',
    programAddr: shortAddr(detail.e3Program),
    requestedBy: shortAddr(detail.requester),
    requestedByLabel: 'Requester',
    requestedTx: detail.requestTxHash,
    requestedAt: `block #${detail.requestBlock.toString()}`,
    requestedBlock: Number(detail.requestBlock),
    currentStage: detail.uiStageIdx,
    summary: meta.question,

    committee: {
      size: committeeSize,
      threshold: sharesRequired,
      selectionSeed: '—',
      selectionTx: shortHash(detail.committeeFinalizedTx ?? '—'),
      drawnAt: detail.committeeFinalizedTx ? 'on-chain' : '—',
      note: 'Identities are sealed. Only the count and threshold are public.',
    },

    fees: {
      requesterDeposit: '—',
      computeFee: '—',
      committeeReward: '—',
      networkFee: '—',
      refundAvailable: '—',
      currency: 'USDC · Sepolia',
    },

    keygen: {
      protocol: 'Threshold BFV · DKG',
      rounds: [
        {
          name: 'Round · DKG aggregation',
          status: detail.committeePublishedTx ? 'complete' : 'pending',
          participants: `${committeeSize} of ${committeeSize}`,
          startedAt: detail.committeeFinalizedTx ? 'after sortition' : '—',
          duration: '—',
          tx: shortHash(detail.committeePublishedTx ?? detail.committeeFinalizedTx ?? '—'),
          note: detail.committeePublishedTx
            ? 'The joint public key has been published on-chain.'
            : 'DKG runs once the committee is finalized.',
        },
      ],
      publicKey:
        detail.committeePublicKey && detail.committeePublicKey !== '0x' + '0'.repeat(64)
          ? `bfv:pk:${shortHash(detail.committeePublicKey)}`
          : '—',
    },

    input: {
      openedAt: detail.inputWindow[0] > 0n ? fmtUtc(detail.inputWindow[0]) : '—',
      closesAt: detail.inputWindow[1] > 0n ? fmtUtc(detail.inputWindow[1]) : '—',
      ballotsReceived,
      firstBallotAt: detail.ballotEvents[0] ? `block #${detail.ballotEvents[0].blockNumber.toString()}` : '—',
      lastBallotAt: detail.ballotEvents[detail.ballotEvents.length - 1]
        ? `block #${detail.ballotEvents[detail.ballotEvents.length - 1].blockNumber.toString()}`
        : '—',
      avgBallotSize: '—',
      ballotCircuit: 'crisp-vote',
    },

    compute: {
      status: detail.uiStageIdx >= 4 ? 'active' : 'pending',
      note:
        detail.uiStageIdx < 4 ? 'Compute begins automatically when the input window closes.' : 'Tally is being computed under encryption.',
      circuit: 'crisp-tally',
      estDuration: '—',
      estGas: '—',
    },

    decryption: {
      status: detail.uiStageIdx >= 5 ? 'active' : 'pending',
      note:
        sharesRequired > 0
          ? `≥ ${sharesRequired} of ${committeeSize} committee members must each publish a partial share.`
          : 'Threshold decryption begins after compute.',
      sharesReceived: detail.uiStageIdx >= 6 ? sharesRequired : 0,
      sharesRequired: sharesRequired || 1,
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

function buildEventLog(d: E3FullDetails) {
  const evs: Array<any> = []
  evs.push({
    t: '—',
    block: Number(d.requestBlock),
    name: 'E3Requested',
    stage: 'Requested',
    tx: shortHash(d.requestTxHash),
    gas: '—',
  })
  if (d.committeeFinalizedTx) {
    evs.push({
      t: '—',
      block: '—',
      name: 'CommitteeFinalized',
      stage: 'Committee Selected',
      tx: shortHash(d.committeeFinalizedTx),
      gas: '—',
    })
  }
  if (d.committeePublishedTx) {
    evs.push({
      t: '—',
      block: '—',
      name: 'CommitteePublished',
      stage: 'Keygen',
      tx: shortHash(d.committeePublishedTx),
      gas: '—',
    })
  }
  d.ballotEvents.slice(0, 5).forEach((b) => {
    evs.push({
      t: '—',
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
      t: '—',
      block: '—',
      name: 'PlaintextOutputPublished',
      stage: 'Published',
      tx: shortHash(d.resultTxHash),
      gas: '—',
    })
  }
  return evs
}

function fmtUtc(ts: bigint): string {
  const d = new Date(Number(ts) * 1000)
  return (
    d.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      timeZone: 'UTC',
    }) +
    ' · ' +
    d.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      timeZone: 'UTC',
      hour12: false,
    }) +
    ' UTC'
  )
}

function fmtDate(ts: bigint): string {
  return new Date(Number(ts) * 1000).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    timeZone: 'UTC',
  })
}

function fmtDuration(seconds: bigint): string {
  const s = Number(seconds)
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.round(s / 60)} min`
  if (s < 86400) return `${Math.round(s / 3600)}h`
  return `${Math.round(s / 86400)} days`
}

// Re-export the mock for callers that want a clean fallback.
export const MOCK_INSPECTOR_DETAIL = E3_DETAILS['E3-0481']
