// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { describe, it, expect, beforeAll, beforeEach, afterEach, vi } from 'vitest'
import { Vote } from '../src/types'
import { SIGNATURE_MESSAGE_HASH, SIGNATURE_MESSAGE, MASK_SIGNATURE } from '../src/constants'
import { generateMerkleProof, getZeroVote } from '../src/utils'
import {
  decodeTally,
  generatePublicKey,
  verifyProof,
  encodeVote,
  generateCircuitInputs,
  executeCircuit,
  computeCiphertextCommitment,
  encryptVote,
} from '../src/vote'
import { publicKeyToAddress, signMessage } from 'viem/accounts'
import { Hex, recoverPublicKey } from 'viem'
import { CRISP_SERVER_URL, ECDSA_PRIVATE_KEY, LEAVES } from './constants'
import { CrispSDK } from '../src/sdk'

describe('Vote', () => {
  let vote: Vote
  let signature: Hex
  let balance: bigint
  let address: string
  let slotAddress: string
  let publicKey: Uint8Array
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

  // Setup the test environment.
  beforeAll(async () => {
    vote = [10n, 0n]
    signature = await signMessage({ message: SIGNATURE_MESSAGE, privateKey: ECDSA_PRIVATE_KEY })
    balance = 100n
    address = publicKeyToAddress(await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature }))
    // Address of the last leaf in the Merkle tree, used for mask votes.
    slotAddress = '0x145B2260E2DAa2965F933A76f5ff5aE3be5A7e5a'
    publicKey = generatePublicKey()
    previousCiphertext = encryptVote(zeroVote, publicKey)
    e3Id = 0
    sdk = new CrispSDK(CRISP_SERVER_URL)
  })

  describe('decodeTally', () => {
    it('Should decode an encoded tally into its decimal representation', () => {
      const tally =
        '0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'

      const decoded = decodeTally(tally, 2)

      expect(decoded[0]).toBe(10000000000n)
      expect(decoded[1]).toBe(30000000000n)
    })
  })

  describe('encodeVote', () => {
    const decodeSegment = (encoded: BigInt64Array, segmentIndex: number, numChoices: number): bigint => {
      const segmentSize = Math.floor(encoded.length / numChoices)
      const start = segmentIndex * segmentSize
      const segment = Array.from(encoded.slice(start, start + segmentSize))
      const binaryString = segment.map((b) => b.toString()).join('')
      const trimmedBinary = binaryString.replace(/^0+/, '') || '0'
      return BigInt('0b' + trimmedBinary)
    }

    it('Should encode votes correctly with 2 choices', () => {
      const encoded = encodeVote([10n, 2n])

      expect(decodeSegment(encoded, 0, 2)).toBe(10n)
      expect(decodeSegment(encoded, 1, 2)).toBe(2n)
    })

    it('Should encode zero votes correctly', () => {
      const encoded = encodeVote([0n, 5n])

      expect(decodeSegment(encoded, 0, 2)).toBe(0n)
      expect(decodeSegment(encoded, 1, 2)).toBe(5n)
    })

    it('Should only contain binary digits (0 or 1)', () => {
      const encoded = encodeVote([255n, 128n])

      expect(Array.from(encoded).every((b) => b === 0n || b === 1n)).toBe(true)
    })

    it('Should encode votes correctly with 3 choices', () => {
      const encoded = encodeVote([10n, 2n, 3n])

      expect(decodeSegment(encoded, 0, 3)).toBe(10n)
      expect(decodeSegment(encoded, 1, 3)).toBe(2n)
      expect(decodeSegment(encoded, 2, 3)).toBe(3n)
    })

    it('Should encode votes correctly with 5 choices', () => {
      const encoded = encodeVote([100n, 50n, 25n, 10n, 5n])

      expect(decodeSegment(encoded, 0, 5)).toBe(100n)
      expect(decodeSegment(encoded, 1, 5)).toBe(50n)
      expect(decodeSegment(encoded, 2, 5)).toBe(25n)
      expect(decodeSegment(encoded, 3, 5)).toBe(10n)
      expect(decodeSegment(encoded, 4, 5)).toBe(5n)
    })

    it('Should handle remainder bits correctly for odd number of choices', () => {
      // With 3 choices, there will be remainder bits at the end
      const encoded = encodeVote([1n, 1n, 1n])

      // All segments should decode correctly
      expect(decodeSegment(encoded, 0, 3)).toBe(1n)
      expect(decodeSegment(encoded, 1, 3)).toBe(1n)
      expect(decodeSegment(encoded, 2, 3)).toBe(1n)

      // Remainder bits (if any) should be zero
      const segmentSize = Math.floor(encoded.length / 3)
      const remainder = encoded.length - segmentSize * 3
      if (remainder > 0) {
        const remainderBits = Array.from(encoded.slice(segmentSize * 3))
        expect(remainderBits.every((b) => b === 0n)).toBe(true)
      }
    })
  })

  describe('generateProof', () => {
    it('Should generate a proof where the output is the new ciphertext', async () => {
      // This test simulates a real vote (i.e. generateVoteProof).

      // Using generateCircuitInputs directly to check the output of the circuit.
      const merkleProof = generateMerkleProof(balance, address, LEAVES)

      const { crispInputs } = await generateCircuitInputs({
        vote,
        publicKey,
        signature,
        messageHash: SIGNATURE_MESSAGE_HASH,
        balance,
        slotAddress: address,
        merkleProof,
        isMaskVote: false,
      })

      const { returnValue } = await executeCircuit(crispInputs)
      const commitment = computeCiphertextCommitment(crispInputs.ct0is, crispInputs.ct1is)

      expect(returnValue).toEqual(commitment)
    })

    it('Should generate a proof where the output is the ciphertext addition if there is a previous ciphertext and 0 vote', async () => {
      // This test simulates a mask vote (i.e. generateMaskVoteProof).

      // Using generateCircuitInputs directly to check the output of the circuit.
      const merkleProof = generateMerkleProof(balance, slotAddress, LEAVES)

      const { crispInputs } = await generateCircuitInputs({
        publicKey,
        balance,
        slotAddress,
        merkleProof,
        vote: zeroVote,
        signature: MASK_SIGNATURE,
        messageHash: SIGNATURE_MESSAGE_HASH,
        previousCiphertext,
        isMaskVote: true,
      })

      const { returnValue } = await executeCircuit(crispInputs)
      const commitment = computeCiphertextCommitment(crispInputs.sum_ct0is, crispInputs.sum_ct1is)

      expect(returnValue).toEqual(commitment)
    })

    it('Should generate a proof where the output is the ciphertext of a 0 vote if there is no previous ciphertext', async () => {
      // This test simulates a mask vote (i.e. generateMaskVoteProof).

      // Using generateCircuitInputs directly to check the output of the circuit.
      const merkleProof = generateMerkleProof(balance, slotAddress, LEAVES)

      const { crispInputs } = await generateCircuitInputs({
        vote: zeroVote,
        publicKey,
        signature: MASK_SIGNATURE,
        messageHash: SIGNATURE_MESSAGE_HASH,
        merkleProof,
        balance,
        slotAddress,
        isMaskVote: true,
      })

      const { returnValue } = await executeCircuit(crispInputs)
      const commitment = computeCiphertextCommitment(crispInputs.ct0is, crispInputs.ct1is)

      expect(returnValue).toEqual(commitment)
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
        numOptions: 2,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

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
        numOptions: 2,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })
})
