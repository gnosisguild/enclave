// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { describe, it, expect, beforeAll, beforeEach, afterEach, vi } from 'vitest'
import { Vote } from '../src/types'
import { SIGNATURE_MESSAGE_HASH, SIGNATURE_MESSAGE, ZERO_VOTE } from '../src/constants'
import { decodeTally, generateBFVKeys, verifyProof, encodeVote, encryptVote, decryptVote } from '../src/vote'
import { signMessage } from 'viem/accounts'
import { Hex } from 'viem'
import { CRISP_SERVER_URL, ECDSA_PRIVATE_KEY, LEAVES } from './constants'
import { CrispSDK } from '../src/sdk'

describe('Vote', () => {
  let vote: Vote
  let signature: Hex
  let balance: bigint
  let slotAddress: string
  let publicKey: Uint8Array
  let secretKey: Uint8Array
  let previousCiphertext: Uint8Array
  let e3Id: number
  let sdk: CrispSDK

  const mockGetPreviousCiphertextResponse = () =>
    ({
      ok: true,
      json: async () => ({ ciphertext: previousCiphertext }),
    }) as Response

  const mockIsSlotEmptyResponse = (isEmpty: boolean) =>
    ({
      ok: true,
      json: async () => ({ is_empty: isEmpty }),
    }) as Response

  beforeEach(() => {
    vi.clearAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  // Setup the test environment.
  beforeAll(async () => {
    vote = { yes: 10n, no: 0n }
    signature = await signMessage({ message: SIGNATURE_MESSAGE, privateKey: ECDSA_PRIVATE_KEY })
    balance = 100n
    // Address of the last leaf in the Merkle tree, used for mask votes.
    slotAddress = '0x145B2260E2DAa2965F933A76f5ff5aE3be5A7e5a'
    const keys = generateBFVKeys()
    publicKey = keys.publicKey
    secretKey = keys.secretKey
    previousCiphertext = encryptVote(ZERO_VOTE, publicKey)
    e3Id = 0
    sdk = new CrispSDK(CRISP_SERVER_URL)
  })

  describe('decodeTally', () => {
    it('Should decode an encoded tally into its decimal representation', () => {
      const tally =
        '0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'

      const decoded = decodeTally(tally)

      expect(decoded.yes).toBe(10000000000n)
      expect(decoded.no).toBe(30000000000n)
    })

    it('Should decode an encoded tally into its decimal representation from a number array', () => {
      const encoded = encodeVote({ yes: 10000000000n, no: 30000000000n })
      const decoded = decodeTally(encoded)

      expect(decoded.yes).toBe(10000000000n)
      expect(decoded.no).toBe(30000000000n)
    })
  })

  describe('encodeVote', () => {
    const decodeHalf = (encoded: number[], isFirstHalf: boolean): number => {
      const halfLength = encoded.length / 2
      const half = isFirstHalf ? encoded.slice(0, halfLength) : encoded.slice(halfLength)
      const binaryString = half.map((b) => b.toString()).join('')
      const trimmedBinary = binaryString.replace(/^0+/, '') || '0'
      return parseInt(trimmedBinary, 2)
    }

    it('Should encode yes vote correctly in the first half', () => {
      const encoded = encodeVote({ yes: 10n, no: 2n })

      expect(decodeHalf(encoded, true)).toBe(10)
      expect(decodeHalf(encoded, false)).toBe(2)
    })

    it('Should encode no vote correctly in the second half', () => {
      const encoded = encodeVote({ yes: 0n, no: 5n })

      expect(decodeHalf(encoded, false)).toBe(5)
    })

    it('Should only contain binary digits (0 or 1)', () => {
      const encoded = encodeVote({ yes: 255n, no: 128n })

      expect(encoded.every((b) => b >= 0 && b <= 1)).toBe(true)
    })
  })

  describe('generateVoteProof', () => {
    it('Should generate a valid vote proof', { timeout: 100000 }, async () => {
      vi.spyOn(global, 'fetch').mockResolvedValueOnce(mockIsSlotEmptyResponse(true))

      const proof = await sdk.generateVoteProof({
        vote,
        publicKey,
        signature,
        merkleLeaves: LEAVES,
        balance,
        messageHash: SIGNATURE_MESSAGE_HASH,
        slotAddress,
        e3Id,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()
      expect(proof.encryptedVote).toBeDefined()

      const decryptedVote = decryptVote(proof.encryptedVote, secretKey)

      expect(decryptedVote).toEqual(vote)

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })

  describe('generateMaskVoteProof', () => {
    it('Should generate a valid mask vote proof when there are no votes in the slot', { timeout: 100000 }, async () => {
      vi.spyOn(global, 'fetch').mockResolvedValueOnce(mockIsSlotEmptyResponse(true))

      const proof = await sdk.generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        merkleLeaves: LEAVES,
        e3Id: 0,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()
      expect(proof.encryptedVote).toBeDefined()

      const decryptedVote = decryptVote(proof.encryptedVote, secretKey)

      expect(decryptedVote).toEqual(ZERO_VOTE)

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })

    it('Should generate a valid mask vote proof when there is a previous vote in the slot', { timeout: 100000 }, async () => {
      vi.spyOn(global, 'fetch')
        .mockResolvedValueOnce(mockIsSlotEmptyResponse(false))
        .mockResolvedValueOnce(mockGetPreviousCiphertextResponse())

      const proof = await sdk.generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        merkleLeaves: LEAVES,
        e3Id: 0,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const decryptedVote = decryptVote(previousCiphertext, secretKey)

      expect(decryptedVote).toEqual(ZERO_VOTE)

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })
})
