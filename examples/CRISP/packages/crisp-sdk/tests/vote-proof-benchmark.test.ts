// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Opt-in benchmark of one client-side CRISP vote proof. It skips unless CRISP_BENCH_PRESET names a
// staged preset. Use scripts/benchmark-vote-proof.sh, which builds and stages that preset, runs this
// test on native bb and on WASM, and restores the previous build.
//
// Every step goes through the SDK: `prepareBallot` encrypts the ballot, and `finishBallotProof`
// attaches the signature and proves user_data_encryption, the `crisp` ballot circuit and the EVM
// fold. The stage times come from the SDK's `onProgress` callback.
//
// Backend: in Node the SDK pins bb.js to WASM (`getBBApi` in src/vote.ts). When BB_BINARY_PATH names
// an existing file, this test asks bb.js for its native backend instead, so that the two backends can
// be compared. Nothing else about proving changes.
//
// Memory: a separate process samples the RSS of this process and its descendants (the native bb
// process is a child) every 100 ms. Proving on WASM blocks the event loop, so a timer in this
// process would not see the peak.
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { existsSync } from 'node:fs'
import { createInterface } from 'node:readline'
import { afterAll, describe, expect, it, vi } from 'vitest'
import { concat, keccak256, numberToHex } from 'viem'
import { privateKeyToAccount, sign } from 'viem/accounts'
import { setCircuits, type CircuitBundle, type CircuitPreset } from '../src/circuits'
import { decryptVote, destroyBBApi, finishBallotProof, generateBFVKeys, prepareBallot, verifyProof } from '../src/vote'
import type { ProofStage, Vote } from '../src/types'
import { generateTestLeaves } from './helpers'

vi.mock('@aztec/bb.js', async (importOriginal) => {
  const bb = await importOriginal<typeof import('@aztec/bb.js')>()
  const { existsSync } = await import('node:fs')
  const native = process.env.BB_BINARY_PATH
  if (!native || !existsSync(native)) return bb

  // Only the backend choice changes. The explicit type makes bb.js fail rather than fall back.
  const Barretenberg = new Proxy(bb.Barretenberg, {
    get(target, prop, receiver) {
      if (prop === 'new') {
        return (options: Parameters<typeof bb.Barretenberg.new>[0] = {}) =>
          target.new({ ...options, backend: bb.BackendType.NativeUnixSocket })
      }
      return Reflect.get(target, prop, receiver)
    },
  })
  return { ...bb, Barretenberg }
})

const preset = process.env.CRISP_BENCH_PRESET as CircuitPreset | undefined
const runs = Number(process.env.CRISP_BENCH_RUNS ?? 2)
const backend = process.env.BB_BINARY_PATH && existsSync(process.env.BB_BINARY_PATH) ? 'native' : 'wasm'

const PRESETS: Record<CircuitPreset, () => Promise<{ loadCircuits: () => Promise<CircuitBundle> }>> = {
  'insecure-64': () => import('../src/presets/insecure-64'),
  'secure-8192': () => import('../src/presets/secure-8192'),
}

// A throwaway key. The circuit proves `slot_address == address(public key)`, so the slot is its address.
const VOTER_KEY = '0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d'
const E3_ID = 1n

const seconds = (ms: number) => (ms / 1000).toFixed(2)
const mb = (kb: number) => Math.round(kb / 1024)

/**
 * Samples the RSS of a process tree from outside it, every 100 ms.
 *
 * Writing `mark` on its stdin returns the peaks since the previous mark, in KB, and resets them.
 */
const SAMPLER = `
const { execFileSync } = require('node:child_process')
const root = Number(process.argv[1])
let peakTree = 0, peakSelf = 0
const sample = () => {
  let out
  try { out = execFileSync('ps', ['-A', '-o', 'pid=,ppid=,rss='], { encoding: 'utf8' }) } catch { return }
  const rows = out.trim().split('\\n').map((l) => l.trim().split(/\\s+/).map(Number))
  const children = new Map()
  for (const [pid, ppid] of rows) children.set(ppid, [...(children.get(ppid) ?? []), pid])
  const rss = new Map(rows.map(([pid, , kb]) => [pid, kb]))
  let tree = 0
  const stack = [root]
  while (stack.length) { const pid = stack.pop(); tree += rss.get(pid) ?? 0; stack.push(...(children.get(pid) ?? [])) }
  peakTree = Math.max(peakTree, tree)
  peakSelf = Math.max(peakSelf, rss.get(root) ?? 0)
}
const timer = setInterval(sample, 100)
sample()
require('node:readline').createInterface({ input: process.stdin }).on('line', () => {
  sample()
  console.log(JSON.stringify({ tree: peakTree, self: peakSelf }))
  peakTree = 0; peakSelf = 0
}).on('close', () => clearInterval(timer))
`

const startSampler = () => {
  const child: ChildProcessWithoutNullStreams = spawn(process.execPath, ['-e', SAMPLER, String(process.pid)])
  const lines = createInterface({ input: child.stdout })[Symbol.asyncIterator]()
  return {
    /** Peak RSS since the previous mark, in KB. */
    mark: async (): Promise<{ tree: number; self: number }> => {
      child.stdin.write('mark\n')
      const { value } = await lines.next()
      return JSON.parse(value as string)
    },
    stop: () => child.stdin.end(),
  }
}

describe.skipIf(!preset)('CRISP vote proof benchmark', () => {
  afterAll(() => {
    destroyBBApi()
  })

  it(`times one vote proof at ${preset}`, async () => {
    expect(PRESETS[preset!], `CRISP_BENCH_PRESET must be one of ${Object.keys(PRESETS).join(', ')}`).toBeDefined()
    setCircuits(await (await PRESETS[preset!]()).loadCircuits())

    const tag = `CRISP_BENCH preset=${preset} backend=${backend}`
    const sampler = startSampler()
    try {
      // Stands in for the committee's public key: a BFV key at the registered preset.
      let start = performance.now()
      const { publicKey, secretKey } = generateBFVKeys()
      const baseline = await sampler.mark()
      console.log(
        `${tag} bb=${process.env.BB_BINARY_PATH ?? 'unset'} keygen=${seconds(performance.now() - start)}s baseline_rss_mb=${mb(baseline.tree)}`,
      )

      const voter = privateKeyToAccount(VOTER_KEY)
      const balance = 10n
      const vote: Vote = [1, 0]
      const merkleLeaves = generateTestLeaves([{ address: voter.address, balance }])

      for (let run = 0; run < runs; run++) {
        const marks: Partial<Record<ProofStage, number>> = {}
        start = performance.now()
        const prepared = await prepareBallot({
          publicKey,
          slotAddress: voter.address,
          isMaskVote: false,
          numOptions: vote.length,
          vote,
          censusMode: 'merkle',
          balance,
          merkleLeaves,
        })
        const prepareMs = performance.now() - start

        // In production the digest is `CRISPProgram.ballotDigest(e3Id, slot, ctCommitment)`. The
        // circuit only proves that the signature covers the digest it is given; it is signed raw,
        // as a wallet signs an EIP-712 digest.
        const digest = keccak256(concat([prepared.ctCommitment, numberToHex(E3_ID, { size: 32 })]))
        const signature = await sign({ hash: digest, privateKey: VOTER_KEY, to: 'hex' })

        start = performance.now()
        const proof = await finishBallotProof(prepared, digest, signature, {
          onProgress: ({ stage }) => {
            marks[stage] = performance.now()
          },
        })
        const end = performance.now()
        const rss = await sampler.mark()

        // `encryption` starts after bb.js is ready (its first start loads the SRS) and covers both
        // circuits' witnesses and the user_data_encryption proof; `fold` covers the fold's witness.
        const setupMs = marks.encryption! - start
        const encryptionMs = marks.ballot! - marks.encryption!
        const ballotMs = marks.fold! - marks.ballot!
        const foldMs = end - marks.fold!
        console.log(
          `${tag} run=${run} prepare=${seconds(prepareMs)}s setup=${seconds(setupMs)}s ` +
            `user_data_encryption=${seconds(encryptionMs)}s crisp=${seconds(ballotMs)}s fold=${seconds(foldMs)}s ` +
            `prove_total=${seconds(end - start)}s total=${seconds(prepareMs + end - start)}s ` +
            `peak_rss_mb=${mb(rss.tree)} peak_rss_main_mb=${mb(rss.self)}`,
        )

        // Checked outside the timings: the ballot is the one encrypted, and the fold proof verifies.
        expect(BigInt(proof.publicInputs[7])).toBe(BigInt(prepared.ctCommitment))
        if (run === 0) {
          expect(decryptVote(proof.encryptedVote, secretKey, vote.length)).toEqual(vote.map(BigInt))
          expect(await verifyProof(proof)).toBe(true)
          // So that the next run's peak does not include the verification.
          await sampler.mark()
        }
      }
    } finally {
      sampler.stop()
    }
  }, 7_200_000)
})
