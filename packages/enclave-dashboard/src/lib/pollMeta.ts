// Off-chain poll metadata. CRISPProgram does not store the human-readable
// question or option labels on-chain — only the ballot/tally circuits and the
// encrypted votes. This map gives each E3 a friendly question/options/context.
// For unknown E3s we synthesize a generic record from the id.

export type PollMeta = {
  question: string
  context: string
  options: Array<{ id: string; label: string }>
  programLabel?: string
}

const META: Record<string, PollMeta> = {
  // Sample seed entry for the demo CRISP deployment. Replace/extend as new
  // E3s are requested on-chain.
  '0': {
    question: 'Should the borough fund a year-round bus lane on Mercer Street, paid for by a small uplift on commercial rates?',
    context: 'Open consultation, citywide. Results are advisory and will be published to the council record.',
    options: [
      { id: 'yes', label: 'Yes, fund the bus lane' },
      { id: 'no', label: 'No, leave the street as it is' },
      { id: 'abs', label: 'Abstain / no opinion' },
    ],
    programLabel: 'CRISP / Binary + Abstain · v0.4.2',
  },
}

export function pollMetaFor(e3Id: bigint): PollMeta {
  const m = META[e3Id.toString()]
  if (m) return m
  return {
    question: `Encrypted poll #${e3Id.toString()}`,
    context: "On-chain encrypted execution. Ballots are sealed on each voter's device; only the aggregate result is decrypted.",
    options: [
      { id: '0', label: 'Option 0' },
      { id: '1', label: 'Option 1' },
      { id: '2', label: 'Option 2' },
    ],
    programLabel: 'CRISP',
  }
}

export function formatE3Id(id: bigint): string {
  return `E3-${id.toString().padStart(4, '0')}`
}

export function shortAddr(addr: string): string {
  if (!addr || addr.length < 12) return addr
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`
}

export function shortHash(hex: string): string {
  if (!hex || hex.length < 14) return hex
  return `${hex.slice(0, 10)}…${hex.slice(-6)}`
}
