// Mock data + canonical stage definitions for the CRISP / Interfold dashboard.

export interface Stage {
  id: string
  label: string
  blurb: string
}

export const STAGES: Stage[] = [
  {
    id: 'requested',
    label: 'Requested',
    blurb: 'An E3 was requested on-chain. The network is preparing to spin up a fresh committee for this poll.',
  },
  {
    id: 'committee',
    label: 'Committee Selected',
    blurb: 'A randomly drawn committee of nodes has been assigned. Their identities are not made public.',
  },
  {
    id: 'keygen',
    label: 'Keygen',
    blurb: 'The committee collaboratively generates a shared encryption key. No single party ever holds the decryption key.',
  },
  {
    id: 'input',
    label: 'Input Window',
    blurb: "Voting is open. Ballots are encrypted on the voter's device and submitted to the network.",
  },
  {
    id: 'compute',
    label: 'Compute',
    blurb: 'The committee tallies the encrypted ballots without ever decrypting an individual vote.',
  },
  {
    id: 'decryption',
    label: 'Decryption',
    blurb: 'Only the final aggregate result is decrypted, and only by a threshold of the committee acting together.',
  },
  {
    id: 'published',
    label: 'Published',
    blurb: 'The result is written on-chain and is now public and verifiable. Individual ballots remain encrypted forever.',
  },
]

export const TODAYS_POLL = {
  id: 'E3-0481',
  question: 'Should the borough fund a year-round bus lane on Mercer Street, paid for by a small uplift on commercial rates?',
  context: 'Open consultation, citywide. Results are advisory and will be published to the council record.',
  options: [
    { id: 'yes', label: 'Yes, fund the bus lane' },
    { id: 'no', label: 'No, leave the street as it is' },
    { id: 'abs', label: 'Abstain / no opinion' },
  ],
  opened: 'May 17, 2026 · 09:00 UTC',
  closes: 'May 21, 2026 · 21:00 UTC',
  ballotCount: 1284,
  result: {
    winner: 'yes',
    totals: { yes: 812, no: 401, abs: 71 } as Record<string, number>,
  },
}

export const HISTORY = [
  {
    id: 'E3-0480',
    question: 'Approve the 2026/27 community-grants programme as scoped?',
    closed: 'May 11, 2026',
    duration: '4 days',
    ballotCount: 2104,
    result: 'Approved · 71%',
  },
  {
    id: 'E3-0479',
    question: 'Adopt the revised noise ordinance for the night-time economy zone?',
    closed: 'Apr 28, 2026',
    duration: '5 days',
    ballotCount: 3318,
    result: 'Adopted · 58%',
  },
  {
    id: 'E3-0478',
    question: 'Rename the South Bridge footpath after Dr. Adaeze Okonkwo?',
    closed: 'Apr 14, 2026',
    duration: '3 days',
    ballotCount: 1772,
    result: 'Approved · 64%',
  },
  {
    id: 'E3-0477',
    question: 'Extend the weekend tram service to 02:00 on the Riverside line for a six-month trial?',
    closed: 'Mar 30, 2026',
    duration: '4 days',
    ballotCount: 4051,
    result: 'Approved · 82%',
  },
  {
    id: 'E3-0476',
    question: 'Approve the cycle-route extension along the canal, subject to environmental review?',
    closed: 'Mar 12, 2026',
    duration: '6 days',
    ballotCount: 2987,
    result: 'Approved · 67%',
  },
  {
    id: 'E3-0475',
    question: 'Should the Friday food market relocate from Union Square to Beech Park for the summer season?',
    closed: 'Feb 27, 2026',
    duration: '5 days',
    ballotCount: 1463,
    result: 'Declined · 54%',
  },
]

export const PULSE = {
  activeNow: 1,
  ballots24h: 612,
  pollsAllTime: 481,
}

export const STAGE_TIMING: Record<string, { remaining: string; sub: string }> = {
  requested: { remaining: 'Starting in moments', sub: 'Committee draw begins shortly' },
  committee: { remaining: '≈ 2 min remaining', sub: 'Drawing a fresh committee' },
  keygen: { remaining: '≈ 4 min remaining', sub: 'Generating the shared key' },
  input: { remaining: '1 day, 14 hours remaining', sub: 'Voting open until May 21, 21:00 UTC' },
  compute: { remaining: '≈ 12 min remaining', sub: 'Tallying under encryption' },
  decryption: { remaining: '≈ 3 min remaining', sub: 'Threshold decrypting the aggregate' },
  published: { remaining: 'Result published', sub: 'On-chain · verifiable' },
}

// ─── E3 INSPECTOR DATA ──────────────────────────────────────────────────────

export const E3_DETAILS: Record<string, any> = {
  'E3-0481': {
    id: 'E3-0481',
    program: 'CRISP / Binary + Abstain · v0.4.2',
    programAddr: '0x9c4a…f201',
    requestedBy: '0x6d2e…a8c1',
    requestedByLabel: 'Mercer Civic Forum',
    requestedTx: '0x7b1f8c2a9d0e4f5b6a3c8d7e1f2a4b5c6d7e8f90a1b2c3d4e5f6a7b8c9d0e1f23',
    requestedAt: 'May 17, 2026 · 08:54:11 UTC',
    requestedBlock: 18_402_117,
    currentStage: 3,
    summary: TODAYS_POLL.question,

    committee: {
      size: 16,
      threshold: 11,
      selectionSeed: '0xf3a2…8e9b (RANDAO + Drand)',
      selectionTx: '0x82a1c0…d7e3',
      drawnAt: 'May 17, 2026 · 08:55:24 UTC',
      note: 'Identities are sealed. Only the count and threshold are public.',
    },

    fees: {
      requesterDeposit: '0.5000 ETH',
      computeFee: '0.0428 ETH',
      committeeReward: '0.0312 ETH',
      networkFee: '0.0084 ETH',
      refundAvailable: '0.4540 ETH',
      currency: 'ETH · settled on Ethereum mainnet',
    },

    keygen: {
      protocol: 'Threshold BFV · DKG (Pedersen variant)',
      rounds: [
        {
          name: 'Round 1 · Commitment broadcast',
          status: 'complete',
          participants: '16 of 16',
          startedAt: '08:56:02 UTC',
          duration: '42s',
          tx: '0x91c4…b203',
          note: 'Each committee member published a public commitment to their share of the secret.',
        },
        {
          name: 'Round 2 · Share distribution',
          status: 'complete',
          participants: '16 of 16',
          startedAt: '08:56:48 UTC',
          duration: '1m 14s',
          tx: '0x91c5…d418',
          note: 'Encrypted shares exchanged pairwise off-chain. On-chain attestations confirm delivery.',
        },
        {
          name: 'Round 3 · Public key finalization',
          status: 'complete',
          participants: '16 of 16',
          startedAt: '08:58:05 UTC',
          duration: '31s',
          tx: '0x91c6…0a77',
          note: 'Joint public key derived and published. The decryption key is held in shares and never reconstructed.',
        },
      ],
      publicKey: 'bfv:pk:0x4a8f1c…29e0  (deg 8192, q 218-bit)',
    },

    input: {
      openedAt: 'May 17, 2026 · 09:00:00 UTC',
      closesAt: 'May 21, 2026 · 21:00:00 UTC',
      ballotsReceived: 1284,
      firstBallotAt: '09:02:18 UTC',
      lastBallotAt: 'ongoing',
      avgBallotSize: '32.1 KB',
      ballotCircuit: 'crisp-vote-bin3 · sha256 0x77ad…12c0',
    },

    compute: {
      status: 'pending',
      note: 'Compute begins automatically when the input window closes. The committee will tally ballots under encryption.',
      circuit: 'crisp-tally-bin3 · sha256 0xbe11…3a04',
      estDuration: '≈ 12 min',
      estGas: '≈ 4.2M gas',
    },

    decryption: {
      status: 'pending',
      note: 'Once compute finishes, ≥ 11 of 16 committee members must each publish a partial decryption of the aggregate ciphertext. The result is then assembled.',
      sharesReceived: 0,
      sharesRequired: 11,
    },

    publication: {
      status: 'pending',
      note: 'The final result will be written to the registry and emit a Published event. Individual ballots remain encrypted forever.',
    },

    events: [
      { t: '08:54:11', block: 18402117, name: 'E3Requested', stage: 'Requested', tx: '0x7b1f…1f23', gas: '184,210' },
      { t: '08:55:24', block: 18402124, name: 'CommitteeSelected', stage: 'Committee Selected', tx: '0x82a1…d7e3', gas: '98,440' },
      { t: '08:56:02', block: 18402128, name: 'KeygenRound1', stage: 'Keygen', tx: '0x91c4…b203', gas: '412,008' },
      { t: '08:56:48', block: 18402132, name: 'KeygenRound2', stage: 'Keygen', tx: '0x91c5…d418', gas: '388,114' },
      { t: '08:58:05', block: 18402138, name: 'KeygenComplete', stage: 'Keygen', tx: '0x91c6…0a77', gas: '256,901' },
      { t: '09:00:00', block: 18402151, name: 'InputWindowOpened', stage: 'Input Window', tx: '0x9a02…34b1', gas: '62,330' },
      { t: '09:02:18', block: 18402163, name: 'BallotSubmitted', stage: 'Input Window', tx: '0xc4d1…a002', gas: '108,440' },
      { t: '—', block: '—', name: 'BallotSubmitted (×1,283 more)', stage: 'Input Window', tx: '—', gas: '—' },
    ],
  },
}

export const E3_LIST = [
  { id: 'E3-0481', label: 'Mercer bus lane consultation', date: 'May 17, 2026', state: 'live' },
  { id: 'E3-0480', label: '2026/27 community-grants programme', date: 'May 11, 2026', state: 'published' },
  { id: 'E3-0479', label: 'Night-time-economy noise ordinance', date: 'Apr 28, 2026', state: 'published' },
  { id: 'E3-0478', label: 'South Bridge footpath renaming', date: 'Apr 14, 2026', state: 'published' },
]
