// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { describe, it, expect, beforeAll, beforeEach, afterEach, vi } from 'vitest'
import { Signature, Vote } from '../src/types'
import { SIGNATURE_MESSAGE_HASH, SIGNATURE_MESSAGE, zeroVote } from '../src/constants'
import { extractSignatureComponents, generateMerkleProof } from '../src/utils'
import {
  decodeTally,
  encryptVote,
  generateVoteProof,
  generateMaskVoteProof,
  generatePublicKey,
  verifyProof,
  encodeVote,
  generateCircuitInputs,
  executeCircuit,
} from '../src/vote'
import { publicKeyToAddress, signMessage } from 'viem/accounts'
import { Hex, recoverPublicKey } from 'viem'
import { CRISP_SERVER_URL, ECDSA_PRIVATE_KEY, LEAVES } from './constants'
import previousCiphertextEncoded from './previous.json'

describe('Vote', () => {
  let vote: Vote
  let signature: Hex
  let balance: bigint
  let address: string
  let slotAddress: string
  let publicKey: Uint8Array
  let derivedSignatureComponents: Signature

  const e3Id = 0

  const mockedPreviousCiphertextResponse = {
    ciphertext: previousCiphertextEncoded,
  }

  const mockedIsSlotEmptyTrueResponse = {
    is_empty: true,
  }

  const mockedIsSlotEmptyFalseResponse = {
    is_empty: false,
  }

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
    derivedSignatureComponents = await extractSignatureComponents(signature, SIGNATURE_MESSAGE_HASH)
    balance = 100n
    address = publicKeyToAddress(await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature }))
    // Address of the last leaf in the Merkle tree, used for mask votes.
    slotAddress = '0x145B2260E2DAa2965F933A76f5ff5aE3be5A7e5a'
    publicKey = generatePublicKey()
  })

  describe('decodeTally', () => {
    it('Should decode an encoded tally into its decimal representation', () => {
      const tally =
        '0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000001000000000000000000000000000000010000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000010000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000300000000000000000000000000000003000000000000000000000000000000030000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000030000000000000003000000000000000300000000000000030000000000000003000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'

      const decoded = decodeTally(tally)

      expect(decoded.yes).toBe(10000000000n)
      expect(decoded.no).toBe(30000000000n)
    })
  })

  describe('encodeVote', () => {
    const decodeHalf = (encoded: BigInt64Array, isFirstHalf: boolean): number => {
      const halfLength = encoded.length / 2
      const half = Array.from(isFirstHalf ? encoded.slice(0, halfLength) : encoded.slice(halfLength))
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

      expect(Array.from(encoded).every((b) => b >= 0n && b <= 1n)).toBe(true)
    })
  })

  describe('generateProof', () => {
    it('Should generate a proof where the output is the new ciphertext', async () => {
      // This test simulates a real vote (i.e. generateVoteProof).

      // Using generateCircuitInputs directly to check the output of the circuit.
      const merkleProof = generateMerkleProof(balance, address, LEAVES)
      const sig = await extractSignatureComponents(signature, SIGNATURE_MESSAGE_HASH)

      const previousCiphertext = encryptVote(zeroVote, publicKey)

      const crispInputs = await generateCircuitInputs({
        vote,
        publicKey,
        signature: sig,
        balance,
        slotAddress: address,
        merkleProof,
        isFirstVote: true,
        previousCiphertext,
      })

      const { returnValue } = await executeCircuit(crispInputs)

      const ct0is = crispInputs.ct0is.flatMap((p) => p.coefficients).map((b) => BigInt(b))
      const ct1is = crispInputs.ct1is.flatMap((p) => p.coefficients).map((b) => BigInt(b))
      const outputCt0is = returnValue[0]
        .flat()
        .flatMap((p) => p.coefficients)
        .map((b) => BigInt(b))
      const outputCt1is = returnValue[1]
        .flat()
        .flatMap((p) => p.coefficients)
        .map((b) => BigInt(b))

      expect([...outputCt0is, ...outputCt1is]).toEqual([...ct0is, ...ct1is])
    })

    it('Should generate a proof where the output is the ciphertext addition if there is a previous ciphertext and 0 vote', async () => {
      // This test simulates a mask vote (i.e. generateMaskVoteProof).

      // Using generateCircuitInputs directly to check the output of the circuit.
      const merkleProof = generateMerkleProof(balance, slotAddress, LEAVES)
      // update to an invalid leaf to fail auth in the circuit
      merkleProof.leaf = 0n
      const crispInputs = await generateCircuitInputs({
        publicKey,
        balance,
        slotAddress,
        isFirstVote: false,
        merkleProof,
        vote: zeroVote,
        signature: derivedSignatureComponents,
        previousCiphertext: new Uint8Array(previousCiphertextEncoded),
      })

      const { returnValue } = await executeCircuit(crispInputs)

      const sumCt0is = crispInputs.sum_ct0is.flatMap((p) => p.coefficients).map((b) => BigInt(b))
      const sumCt1is = crispInputs.sum_ct1is.flatMap((p) => p.coefficients).map((b) => BigInt(b))
      const outputSumCt0is = returnValue[0]
        .flat()
        .flatMap((p) => p.coefficients)
        .map((b) => BigInt(b))
      const outputSumCt1is = returnValue[1]
        .flat()
        .flatMap((p) => p.coefficients)
        .map((b) => BigInt(b))

      expect([...outputSumCt0is, ...outputSumCt1is]).toEqual([...sumCt0is, ...sumCt1is])
    })

    it('Should generate a proof where the output is the ciphertext of a 0 vote if there is no previous ciphertext', async () => {
      // This test simulates a mask vote (i.e. generateMaskVoteProof).

      // Using generateCircuitInputs directly to check the output of the circuit.
      const merkleProof = generateMerkleProof(balance, slotAddress, LEAVES)
      const crispInputs = await generateCircuitInputs({
        vote: zeroVote,
        publicKey,
        signature: derivedSignatureComponents,
        merkleProof,
        balance,
        slotAddress,
        isFirstVote: true,
        previousCiphertext: encryptVote(vote, publicKey),
      })

      const { returnValue } = await executeCircuit(crispInputs)

      const ct0is = crispInputs.ct0is.flatMap((p) => p.coefficients).map((b) => BigInt(b))
      const ct1is = crispInputs.ct1is.flatMap((p) => p.coefficients).map((b) => BigInt(b))
      const outputCt0is = returnValue[0]
        .flat()
        .flatMap((p) => p.coefficients)
        .map((b) => BigInt(b))
      const outputCt1is = returnValue[1]
        .flat()
        .flatMap((p) => p.coefficients)
        .map((b) => BigInt(b))

      expect([...outputCt0is, ...outputCt1is]).toEqual([...ct0is, ...ct1is])
    })
  })

  describe('generateVoteProof', () => {
    it('Should generate a valid vote proof', { timeout: 100000 }, async () => {
      const mockIsSlotEmptyResponse = {
        ok: true,
        json: async () => mockedIsSlotEmptyTrueResponse,
      } as Response

      vi.spyOn(global, 'fetch').mockResolvedValue(mockIsSlotEmptyResponse)

      const proof = await generateVoteProof({
        vote,
        publicKey,
        signature,
        merkleLeaves: LEAVES,
        balance,
        messageHash: SIGNATURE_MESSAGE_HASH,
        serverUrl: CRISP_SERVER_URL,
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
      const mockIsSlotEmptyResponse = {
        ok: true,
        json: async () => mockedIsSlotEmptyTrueResponse,
      } as Response

      vi.spyOn(global, 'fetch').mockResolvedValueOnce(mockIsSlotEmptyResponse)

      const proof = await generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        merkleLeaves: LEAVES,
        e3Id: 0,
        serverUrl: CRISP_SERVER_URL,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })

    it('Should generate a valid mask vote proof when there is a previous vote in the slot', { timeout: 100000 }, async () => {
      const mockIsSlotEmptyResponse = {
        ok: true,
        json: async () => mockedIsSlotEmptyFalseResponse,
      } as Response

      const mockFetchResponse = {
        ok: true,
        json: async () => mockedPreviousCiphertextResponse,
      } as Response

      vi.spyOn(global, 'fetch').mockResolvedValueOnce(mockIsSlotEmptyResponse).mockResolvedValueOnce(mockFetchResponse)

      const proof = await generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        merkleLeaves: LEAVES,
        e3Id: 0,
        serverUrl: CRISP_SERVER_URL,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })
})
