// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/**
 * Vote encoding and BFV encryption for the CRISP voting protocol.
 *
 * Encodes vote choices (numbers per option) into polynomial coefficient arrays
 * suitable for BFV homomorphic encryption. Each choice is represented as a
 * segment of binary digits, padded to fit the polynomial degree. Supports
 * encoding, encryption, decryption, and tally decoding.
 */

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import { toBinary, numberArrayToBigInt64Array, decodeBytesToNumbers } from './utils'
import { MAX_VOTE_BITS } from './constants'
import { hexToBytes } from 'viem'
import type { Hex } from 'viem'
import type { Vote } from './types'

let _zkInputsGenerator: InstanceType<typeof ZKInputsGenerator> | null = null

/**
 * Returns the singleton ZK inputs generator instance (lazily initialized).
 */
export const getZkInputsGenerator = () => {
  if (!_zkInputsGenerator) {
    _zkInputsGenerator = ZKInputsGenerator.withDefaults()
  }
  return _zkInputsGenerator
}

/**
 * Encodes vote choices into a polynomial coefficient array for BFV encryption.
 * Each choice is split into a segment of binary digits; segments are padded
 * to align with the polynomial degree.
 *
 * @param vote - Array of numeric values per choice (e.g. [10, 5] for 2 options)
 * @returns Array of 0s and 1s representing coefficients
 * @throws If vote has fewer than 2 choices or any value exceeds max bits
 */
export const encodeVote = (vote: Vote): number[] => {
  if (vote.length < 2) {
    throw new Error('Vote must have at least two choices')
  }

  const bfvParams = getZkInputsGenerator().getBFVParams()
  const degree = bfvParams.degree
  const n = vote.length

  const segmentSize = Math.floor(degree / n)
  const maxBits = Math.min(segmentSize, MAX_VOTE_BITS)
  const maxValue = 2 ** maxBits - 1
  const voteArray: number[] = []

  for (let choiceIdx = 0; choiceIdx < n; choiceIdx += 1) {
    const value = choiceIdx < vote.length ? vote[choiceIdx] : 0

    if (value > maxValue) {
      throw new Error(`Vote value for choice ${choiceIdx} exceeds maximum (${maxValue})`)
    }

    const binary = toBinary(value).split('')

    for (let i = 0; i < segmentSize; i += 1) {
      const offset = segmentSize - binary.length
      voteArray.push(i < offset ? 0 : parseInt(binary[i - offset]))
    }
  }

  const remainder = degree - segmentSize * n
  for (let i = 0; i < remainder; i++) {
    voteArray.push(0)
  }

  return voteArray
}

/**
 * Encrypts an encoded vote using BFV homomorphic encryption.
 *
 * @param vote - Vote choices to encrypt
 * @param publicKey - BFV public key
 * @returns Encrypted ciphertext
 */
export const encryptVote = (vote: Vote, publicKey: Uint8Array): Uint8Array => {
  const encodedVote = encodeVote(vote)

  return getZkInputsGenerator().encryptVote(publicKey, numberArrayToBigInt64Array(encodedVote))
}

/**
 * Decodes raw tally bytes (or hex string) into vote values per choice.
 * Expects the same segment layout as used in encodeVote.
 *
 * @param tallyBytes - Hex string or array of decoded numbers from tally/decryption
 * @param numChoices - Number of vote options
 * @returns Vote array with one value per choice
 */
export const decodeTally = (tallyBytes: string | number[], numChoices: number): Vote => {
  if (typeof tallyBytes === 'string') {
    const hexString = tallyBytes.startsWith('0x') ? tallyBytes : `0x${tallyBytes}`
    tallyBytes = decodeBytesToNumbers(hexToBytes(hexString as Hex))
  }

  if (numChoices <= 0) {
    throw new Error('Number of choices must be positive')
  }

  const segmentSize = Math.floor(tallyBytes.length / numChoices)
  const effectiveSize = Math.min(segmentSize, MAX_VOTE_BITS)
  const results: Vote = []

  for (let choiceIdx = 0; choiceIdx < numChoices; choiceIdx++) {
    const segmentStart = choiceIdx * segmentSize
    const readStart = segmentStart + segmentSize - effectiveSize
    const segment = tallyBytes.slice(readStart, readStart + effectiveSize)

    let value = 0
    for (let i = 0; i < segment.length; i++) {
      const weight = 2 ** (segment.length - 1 - i)
      value += segment[i] * weight
    }

    results.push(value)
  }

  return results
}

/**
 * Decrypts a BFV-encrypted vote and decodes it to vote values.
 *
 * @param ciphertext - Encrypted vote
 * @param secretKey - BFV secret key
 * @param numChoices - Number of vote options
 * @returns Decrypted vote array
 */
export const decryptVote = (ciphertext: Uint8Array, secretKey: Uint8Array, numChoices: number): Vote => {
  const decryptedVote = getZkInputsGenerator().decryptVote(secretKey, ciphertext)

  return decodeTally(
    Array.from(decryptedVote).map((v) => Number(v)),
    numChoices,
  )
}

/**
 * Generates a BFV keypair for vote encryption and decryption.
 *
 * @returns Object with secretKey and publicKey as Uint8Arrays
 */
export const generateBFVKeys = (): { secretKey: Uint8Array; publicKey: Uint8Array } => {
  return getZkInputsGenerator().generateKeys()
}
