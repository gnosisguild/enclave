// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { describe, it, expect, beforeAll, beforeEach, afterEach, vi } from 'vitest'
import { Vote } from '../src/types'
import { SIGNATURE_MESSAGE_HASH, SIGNATURE_MESSAGE } from '../src/constants'
import { getZeroVote } from '../src/utils'
import { decodeTally, verifyProof, encodeVote, generateBFVKeys, encryptVote, decryptVote } from '../src/vote'
import { publicKeyToAddress, signMessage } from 'viem/accounts'
import { Hex, recoverPublicKey } from 'viem'
import { CRISP_SERVER_URL, ECDSA_PRIVATE_KEY, SLOT_ADDRESS } from './constants'
import { CrispSDK } from '../src/sdk'
import { generateTestLeaves } from './helpers'

describe('Vote', () => {
  let vote: Vote
  let signature: Hex
  let balance: bigint
  let address: string
  let leaves: bigint[]
  let publicKey: Uint8Array
  let secretKey: Uint8Array
  let previousCiphertext: Uint8Array
  let e3Id: number
  let sdk: CrispSDK

  const zeroVote = getZeroVote(2)

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

  beforeAll(async () => {
    vote = [10, 0, 0]
    signature = await signMessage({ message: SIGNATURE_MESSAGE, privateKey: ECDSA_PRIVATE_KEY })
    balance = 10n
    address = publicKeyToAddress(await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature }))
    leaves = generateTestLeaves([
      { address, balance },
      { address: SLOT_ADDRESS, balance },
    ])
    const keys = generateBFVKeys()
    publicKey = keys.publicKey
    secretKey = keys.secretKey
    previousCiphertext = encryptVote(zeroVote, publicKey)
    e3Id = 0
    sdk = new CrispSDK(CRISP_SERVER_URL)
  })

  describe('decodeTally', () => {
    it('Should decode an encoded tally into its decimal representation', () => {
      const tally =
        '0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'

      const decoded = decodeTally(tally, 2)

      expect(decoded[0]).toBe(10000000000)
      expect(decoded[1]).toBe(30000000000)
    })
  })

  describe('encodeVote', () => {
    it('Should fail when the number of choices is less than 2', () => {
      expect(() => encodeVote([10])).toThrow('Vote must have at least two choices')
      expect(() => encodeVote([])).toThrow('Vote must have at least two choices')
    })

    it('Should encode votes correctly with 2 choices', () => {
      const encoded = encodeVote([10, 2])
      const decoded = decodeTally(encoded, 2)

      expect(decoded[0]).toBe(10)
      expect(decoded[1]).toBe(2)
    })

    it('Should encode zero votes correctly', () => {
      const encoded = encodeVote([0, 5])
      const decoded = decodeTally(encoded, 2)

      expect(decoded[0]).toBe(0)
      expect(decoded[1]).toBe(5)
    })

    it('Should only contain binary digits (0 or 1)', () => {
      const encoded = encodeVote([255, 128])

      expect(Array.from(encoded).every((b) => b === 0 || b === 1)).toBe(true)
    })

    it('Should encode votes correctly with 3 choices', () => {
      const encoded = encodeVote([10, 2, 3])
      const decoded = decodeTally(encoded, 3)

      expect(decoded[0]).toBe(10)
      expect(decoded[1]).toBe(2)
      expect(decoded[2]).toBe(3)
    })

    it('Should encode votes correctly with 5 choices', () => {
      const encoded = encodeVote([100, 50, 25, 10, 5])
      const decoded = decodeTally(encoded, 5)

      expect(decoded[0]).toBe(100)
      expect(decoded[1]).toBe(50)
      expect(decoded[2]).toBe(25)
      expect(decoded[3]).toBe(10)
      expect(decoded[4]).toBe(5)
    })

    it('Should handle remainder bits correctly for odd number of choices', () => {
      // With 3 choices, there will be remainder bits at the end
      const encoded = encodeVote([1, 1, 1])
      const decoded = decodeTally(encoded, 3)

      // All segments should decode correctly
      expect(decoded[0]).toBe(1)
      expect(decoded[1]).toBe(1)
      expect(decoded[2]).toBe(1)

      // Remainder bits (if any) should be zero
      const segmentSize = Math.floor(encoded.length / 3)
      const remainder = encoded.length - segmentSize * 3
      if (remainder > 0) {
        const remainderBits = Array.from(encoded.slice(segmentSize * 3))
        expect(remainderBits.every((b) => b === 0)).toBe(true)
      }
    })
  })

  describe('generateVoteProof', () => {
    it('Should generate a valid vote proof', { timeout: 300000 }, async () => {
      vi.spyOn(global, 'fetch').mockResolvedValueOnce(mockIsSlotEmptyResponse(true))

      const proof = await sdk.generateVoteProof({
        vote,
        publicKey,
        signature,
        merkleLeaves: leaves,
        balance,
        messageHash: SIGNATURE_MESSAGE_HASH,
        slotAddress: SLOT_ADDRESS,
        e3Id,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()
      expect(proof.encryptedVote).toBeDefined()

      const decryptedVote = decryptVote(proof.encryptedVote, secretKey, vote.length)

      expect(decryptedVote).toEqual(vote)

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })

  describe('generateMaskVoteProof', () => {
    it('Should generate a valid mask vote proof when there are no votes in the slot', { timeout: 300000 }, async () => {
      vi.spyOn(global, 'fetch').mockResolvedValueOnce(mockIsSlotEmptyResponse(true))

      const proof = await sdk.generateMaskVoteProof({
        balance,
        slotAddress: SLOT_ADDRESS,
        publicKey,
        merkleLeaves: leaves,
        e3Id: 0,
        numOptions: 2,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()
      expect(proof.encryptedVote).toBeDefined()

      const decryptedVote = decryptVote(proof.encryptedVote, secretKey, 2)

      expect(decryptedVote).toEqual(zeroVote)

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })

    it('Should generate a valid mask vote proof when there is a previous vote in the slot', { timeout: 300000 }, async () => {
      vi.spyOn(global, 'fetch')
        .mockResolvedValueOnce(mockIsSlotEmptyResponse(false))
        .mockResolvedValueOnce(mockGetPreviousCiphertextResponse())

      const proof = await sdk.generateMaskVoteProof({
        balance,
        slotAddress: SLOT_ADDRESS,
        publicKey,
        merkleLeaves: leaves,
        e3Id: 0,
        numOptions: 2,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const decryptedVote = decryptVote(previousCiphertext, secretKey, 2)

      expect(decryptedVote).toEqual(zeroVote)

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })
})
