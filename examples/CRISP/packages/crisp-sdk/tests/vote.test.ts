// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, it, expect, beforeAll } from 'vitest'
import { SIGNATURE_MESSAGE, generateMerkleProof, hashLeaf, IVote, IMerkleProof } from '../src'
import { decodeTally, encryptVote, generateVoteProof, generateMaskVoteProof, generatePublicKey, verifyProof } from '../src/vote'
import { signMessage } from 'viem/accounts'
import { Hex } from 'viem'
import { ECDSA_PRIVATE_KEY } from './constants'

describe('Vote', () => {
  let vote: IVote
  let signature: Hex
  let balance: bigint
  let slotAddress: string
  let merkleProof: IMerkleProof
  let publicKey: Uint8Array
  let previousCiphertext: Uint8Array

  // Setup the test environment.
  beforeAll(async () => {
    vote = { yes: 10n, no: 0n }
    signature = await signMessage({ message: SIGNATURE_MESSAGE, privateKey: ECDSA_PRIVATE_KEY })
    balance = 100n
    slotAddress = '0x58Ce9Da2B075732302AE95175c48891b305A40A4'
    merkleProof = generateMerkleProof(balance, slotAddress, [0n, 1n, 2n, 3n, hashLeaf(slotAddress, balance)])
    publicKey = generatePublicKey()
    previousCiphertext = encryptVote(vote, publicKey)
  })

  describe('decode tally', () => {
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

  describe('generateVoteProof', () => {
    it('Should generate a valid vote proof', { timeout: 100000 }, async () => {
      const voteProofInputs = {
        vote,
        publicKey,
        previousCiphertext,
        signature,
        merkleProof,
        balance,
        slotAddress,
      }

      const proof = await generateVoteProof(voteProofInputs)

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })

  describe('generateMaskVoteProof', () => {
    it('Should generate a valid mask vote proof', { timeout: 100000 }, async () => {
      const proof = await generateMaskVoteProof({
        balance,
        slotAddress,
        publicKey,
        previousCiphertext,
        merkleProof,
      })

      expect(proof).toBeDefined()
      expect(proof.proof).toBeDefined()
      expect(proof.publicInputs).toBeDefined()

      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })
  })
})
