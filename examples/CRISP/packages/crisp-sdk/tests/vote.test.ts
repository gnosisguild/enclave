// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, it, expect, beforeAll } from 'vitest'
import { Vote } from '../src/types'
import { MASK_SIGNATURE, SIGNATURE_MESSAGE_HASH, SIGNATURE_MESSAGE } from '../src/constants'
import { generateMerkleProof } from '../src/utils'
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
import { ECDSA_PRIVATE_KEY, LEAVES } from './constants'

describe('Vote', () => {
  let vote: Vote
  let signature: Hex
  let balance: bigint
  let address: string
  let slotAddress: string
  let publicKey: Uint8Array
  let previousCiphertext: Uint8Array

  // Setup the test environment.
  beforeAll(async () => {
    vote = { yes: 10n, no: 0n }
    signature = await signMessage({ message: SIGNATURE_MESSAGE, privateKey: ECDSA_PRIVATE_KEY })
    balance = 100n
    address = publicKeyToAddress(await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature }))
    // Address of the last leaf in the Merkle tree, used for mask votes.
    slotAddress = '0x145B2260E2DAa2965F933A76f5ff5aE3be5A7e5a'
    publicKey = generatePublicKey()
    previousCiphertext = encryptVote(vote, publicKey)
  })

  describe('decodeTally', () => {
    it('Should decode an encoded tally into its decimal representation', () => {
      const tally = [
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '5',
        '0',
        '2',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '0',
        '1',
      ]

      const decoded = decodeTally(tally)

      expect(decoded.yes).toBe(22n)
      expect(decoded.no).toBe(1n)
    })
  })

  describe('encodeVote', () => {
    const decodeHalf = (encoded: BigInt64Array, isFirstHalf: boolean): bigint => {
      const halfLength = encoded.length / 2
      const half = Array.from(isFirstHalf ? encoded.slice(0, halfLength) : encoded.slice(halfLength))
      const binaryString = half.map((b) => b.toString()).join('')
      const trimmedBinary = binaryString.replace(/^0+/, '') || '0'
      return BigInt('0b' + trimmedBinary)
    }

    it('Should encode yes vote correctly in the first half', () => {
      const encoded = encodeVote({ yes: 10n, no: 0n })

      expect(decodeHalf(encoded, true)).toBe(10n)
    })

    it('Should encode no vote correctly in the second half', () => {
      const encoded = encodeVote({ yes: 0n, no: 5n })

      expect(decodeHalf(encoded, false)).toBe(5n)
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
      const crispInputs = await generateCircuitInputs({
        vote,
        publicKey,
        signature,
        balance,
        slotAddress: address,
        merkleProof,
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
      const crispInputs = await generateCircuitInputs({
        vote: { yes: 0n, no: 0n },
        publicKey,
        previousCiphertext,
        signature: MASK_SIGNATURE,
        merkleProof,
        balance,
        slotAddress,
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
        vote: { yes: 0n, no: 0n },
        publicKey,
        signature: MASK_SIGNATURE,
        merkleProof,
        balance,
        slotAddress,
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
      const proof = await generateVoteProof({
        vote,
        publicKey,
        signature,
        merkleLeaves: LEAVES,
        balance,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })

  describe('generateMaskVoteProof', () => {
    it('Should generate a valid mask vote proof with a previous ciphertext', { timeout: 100000 }, async () => {
      const proof = await generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        previousCiphertext,
        merkleLeaves: LEAVES,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })

    it('Should generate a valid mask vote proof without a previous ciphertext', { timeout: 100000 }, async () => {
      const proof = await generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        merkleLeaves: LEAVES,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })
})
