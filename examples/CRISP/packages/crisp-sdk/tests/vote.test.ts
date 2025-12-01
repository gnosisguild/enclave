// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, it, expect } from 'vitest'
import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import {
  calculateValidIndicesForPlaintext,
  decodeTally,
  encodeVote,
  encryptVoteAndGenerateCRISPInputs,
  generateMaskVote,
  generateProof,
  generateProofWithReturnValue,
  getCircuitOutputValue,
  validateVote,
  verifyProof,
} from '../src/vote'
import { BFVParams, VotingMode } from '../src/types'
import { generateMerkleProof, hashLeaf, MAXIMUM_VOTE_VALUE } from '../src'

import { LEAVES, merkleProof, MESSAGE, SIGNATURE, testAddress, VOTE, votingPowerLeaf } from './constants'
import { privateKeyToAccount } from 'viem/accounts'
import { compareCoefficientsArrays } from './utils'

describe('Vote', () => {
  const votingPower = 10n

  let zkInputsGenerator = ZKInputsGenerator.withDefaults()
  const DEFAULT_BFV_PARAMS = zkInputsGenerator.getBFVParams() as BFVParams

  let publicKey = zkInputsGenerator.generatePublicKey()
  const previousCiphertext = zkInputsGenerator.encryptVote(publicKey, new BigInt64Array([0n]))

  describe('encodeVote', () => {
    it('should work for valid votes', () => {
      const encoded = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)
      expect(encoded.length).toBe(DEFAULT_BFV_PARAMS.degree)
    })
  })

  describe('decode tally', () => {
    it('should decode correctly', () => {
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

      const decoded = decodeTally(tally, VotingMode.GOVERNANCE)
      expect(decoded.yes).toBe(22n)
      expect(decoded.no).toBe(1n)
    })
  })

  describe('validateVote', () => {
    const validVote = { yes: 10n, no: 0n }
    const invalidVote = { yes: 5n, no: 5n }

    it('should throw an error for invalid GOVERNANCE votes', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, invalidVote, votingPower)
      }).toThrow('Invalid vote for GOVERNANCE mode: cannot spread votes between options')
    })
    it('should work for valid GOVERNANCE votes', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, validVote, votingPower)
      }).not.toThrow()
    })
    it('should not throw when vote does not exceed the maximum value supported', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, { yes: 10n, no: 0n }, votingPower)
      }).not.toThrow()
    })
    it('should throw when the vote exceeds the maximum value supported', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, { yes: MAXIMUM_VOTE_VALUE + 1n, no: 0n }, MAXIMUM_VOTE_VALUE + 1n)
      }).toThrow('Invalid vote for GOVERNANCE mode: vote exceeds maximum allowed value')
    })
  })

  describe('calculateValidIndicesForPlaintext', () => {
    it('should return the correct indices', () => {
      const degree = 8192
      const totalVotingPower = 100n

      // bitsNeeded = 7 -> 1100100 = 100 in binary
      // half length = 4096
      // first valid index for yes 4096 - 7 = 4089
      // first valid index for no 8192 - 7 = 8185
      expect(calculateValidIndicesForPlaintext(totalVotingPower, degree)).toEqual({
        yesIndex: 4089,
        noIndex: 8185,
      })
    })
    it('should throw if voting power is too high for degree', () => {
      const degree = 16
      const totalVotingPower = 10000n

      expect(() => {
        calculateValidIndicesForPlaintext(totalVotingPower, degree)
      }).toThrow('Total voting power exceeds maximum representable votes for the given degree')
    })
    it('should throw when the degree is negative', () => {
      expect(() => {
        calculateValidIndicesForPlaintext(10n, -16)
      }).toThrow('Degree must be a positive even number')
    })
    it('should throw when the degree is not even', () => {
      expect(() => {
        calculateValidIndicesForPlaintext(10n, 15)
      }).toThrow('Degree must be a positive even number')
    })
  })

  describe('encryptVoteAndGenerateCRISPInputs', () => {
    it('should encrypt a vote and generate the circuit inputs', async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)
      const crispInputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature: SIGNATURE,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: votingPowerLeaf,
        slotAddress: testAddress,
        isFirstVote: false,
      })

      expect(crispInputs.prev_ct0is).toBeInstanceOf(Array)
      expect(crispInputs.prev_ct1is).toBeInstanceOf(Array)
      expect(crispInputs.sum_ct0is).toBeInstanceOf(Array)
      expect(crispInputs.sum_ct1is).toBeInstanceOf(Array)
      expect(crispInputs.sum_r0is).toBeInstanceOf(Array)
      expect(crispInputs.sum_r1is).toBeInstanceOf(Array)
      expect(crispInputs.params).toBeInstanceOf(Object)
      expect(crispInputs.ct0is).toBeInstanceOf(Array)
      expect(crispInputs.ct1is).toBeInstanceOf(Array)
      expect(crispInputs.pk0is).toBeInstanceOf(Array)
      expect(crispInputs.pk1is).toBeInstanceOf(Array)
      expect(crispInputs.r1is).toBeInstanceOf(Array)
      expect(crispInputs.r2is).toBeInstanceOf(Array)
      expect(crispInputs.p1is).toBeInstanceOf(Array)
      expect(crispInputs.hashed_message).toBeInstanceOf(Array)
      expect(crispInputs.public_key_x).toBeInstanceOf(Array)
      expect(crispInputs.public_key_y).toBeInstanceOf(Array)
      expect(crispInputs.signature).toBeInstanceOf(Array)
      expect(crispInputs.merkle_proof_indices).toBeDefined()
      expect(crispInputs.merkle_proof_siblings).toBeDefined()
      expect(crispInputs.merkle_proof_length).toBeDefined()
      expect(crispInputs.merkle_root).toBeDefined()
      expect(crispInputs.balance).toBe(votingPowerLeaf.toString())
    })
  })

  describe('generateMaskVote', () => {
    it('should generate a mask vote and the right circuit inputs', async () => {
      const crispInputs = await generateMaskVote(publicKey, previousCiphertext, merkleProof.proof.root, testAddress, false)

      expect(crispInputs.prev_ct0is).toBeInstanceOf(Array)
      expect(crispInputs.prev_ct1is).toBeInstanceOf(Array)
      expect(crispInputs.sum_ct0is).toBeInstanceOf(Array)
      expect(crispInputs.sum_ct1is).toBeInstanceOf(Array)
      expect(crispInputs.sum_r0is).toBeInstanceOf(Array)
      expect(crispInputs.sum_r1is).toBeInstanceOf(Array)
      expect(crispInputs.params).toBeInstanceOf(Object)
      expect(crispInputs.ct0is).toBeInstanceOf(Array)
      expect(crispInputs.ct1is).toBeInstanceOf(Array)
      expect(crispInputs.pk0is).toBeInstanceOf(Array)
      expect(crispInputs.pk1is).toBeInstanceOf(Array)
      expect(crispInputs.r1is).toBeInstanceOf(Array)
      expect(crispInputs.r2is).toBeInstanceOf(Array)
      expect(crispInputs.p1is).toBeInstanceOf(Array)
      expect(crispInputs.hashed_message).toBeInstanceOf(Array)
      expect(crispInputs.public_key_x).toBeInstanceOf(Array)
      expect(crispInputs.public_key_y).toBeInstanceOf(Array)
      expect(crispInputs.signature).toBeInstanceOf(Array)
      expect(crispInputs.merkle_proof_indices).toBeDefined()
      expect(crispInputs.merkle_proof_siblings).toBeDefined()
      expect(crispInputs.merkle_proof_length).toBeDefined()
      expect(crispInputs.merkle_root).toBeDefined()
      expect(crispInputs.balance).toBeDefined()
    })
  })

  describe('generateProof/verifyProof', () => {
    it('should generate a proof for a voter and verify it', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)

      // hardhat default private key
      const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
      const account = privateKeyToAccount(privateKey)
      const signature = await account.signMessage({ message: MESSAGE })
      const leaf = hashLeaf(account.address.toLowerCase(), votingPowerLeaf.toString())
      const leaves = [...LEAVES, leaf]
      const merkleProof = generateMerkleProof(0n, votingPowerLeaf, account.address.toLowerCase(), leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: votingPowerLeaf,
        slotAddress: account.address.toLowerCase(),
        isFirstVote: false,
      })

      const proof = await generateProof(inputs)
      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })

    it('should generate a proof for a masking user and verify it', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)
      const zkInputsGenerator = ZKInputsGenerator.withDefaults()
      const vote = BigInt64Array.from(encodedVote.map(BigInt))
      const encryptedVote = zkInputsGenerator.encryptVote(publicKey, vote)

      let maskVote = await generateMaskVote(publicKey, encryptedVote, merkleProof.proof.root, testAddress, false)

      const proof = await generateProof(maskVote)
      const isValid = await verifyProof(proof)

      expect(isValid).toBe(true)
    })

    it('should return ciphertext if masking a vote and it is the first operation on the slot', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)
      const zkInputsGenerator = ZKInputsGenerator.withDefaults()
      const vote = BigInt64Array.from(encodedVote.map(BigInt))
      const encryptedVote = zkInputsGenerator.encryptVote(publicKey, vote)

      let maskVote = await generateMaskVote(publicKey, encryptedVote, merkleProof.proof.root, testAddress, true)

      const { returnValue } = await generateProofWithReturnValue(maskVote)

      expect(compareCoefficientsArrays(maskVote.ct0is, (returnValue as any[])[0])).toBe(true)
      expect(compareCoefficientsArrays(maskVote.ct1is, (returnValue as any[])[1])).toBe(true)
    })

    it('should return the sum if masking a vote and it is not the first operation on the slot', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)
      const zkInputsGenerator = ZKInputsGenerator.withDefaults()
      const vote = BigInt64Array.from(encodedVote.map(BigInt))
      const encryptedVote = zkInputsGenerator.encryptVote(publicKey, vote)

      let maskVote = await generateMaskVote(publicKey, encryptedVote, merkleProof.proof.root, testAddress, false)

      const { returnValue } = await generateProofWithReturnValue(maskVote)

      expect(compareCoefficientsArrays(maskVote.sum_ct0is, (returnValue as any[])[0])).toBe(true)
      expect(compareCoefficientsArrays(maskVote.sum_ct1is, (returnValue as any[])[1])).toBe(true)
    })

    it('should throw when the signature is invalid and it is a vote (no masking)', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)

      // hardhat default private key
      const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
      const account = privateKeyToAccount(privateKey)
      const signature = await account.signMessage({ message: MESSAGE })
      const leaf = hashLeaf(account.address.toLowerCase(), votingPowerLeaf.toString())
      const leaves = [...LEAVES, leaf]
      const merkleProof = generateMerkleProof(0n, votingPowerLeaf, account.address.toLowerCase(), leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: votingPowerLeaf,
        slotAddress: account.address.toLowerCase(),
        isFirstVote: false,
      })

      // invalidate signature
      inputs.signature[0] = '0'

      await expect(getCircuitOutputValue(inputs)).rejects.toThrow()
    })

    it('should throw when the merkle tree inclusion proof is invalid and it is a vote (no masking)', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)

      // hardhat default private key
      const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
      const account = privateKeyToAccount(privateKey)
      const signature = await account.signMessage({ message: MESSAGE })
      const leaf = hashLeaf(account.address.toLowerCase(), votingPowerLeaf.toString())
      const leaves = [...LEAVES, leaf]
      const merkleProof = generateMerkleProof(0n, votingPowerLeaf, account.address.toLowerCase(), leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: votingPowerLeaf,
        slotAddress: account.address.toLowerCase(),
        isFirstVote: false,
      })

      // invalidate merkle root
      inputs.merkle_root = '0'

      await expect(getCircuitOutputValue(inputs)).rejects.toThrow()
    })

    it('should succeed when the vote is the maximum value supported', { timeout: 100000 }, async () => {
      const MAXIMUM_VOTE_VALUE = BigInt(Math.pow(2, 28) - 1) // 268,435,455
      const votingPowerLeaf = MAXIMUM_VOTE_VALUE // Balance at the limit

      // Vote exactly at the maximum
      const VOTE = {
        yes: MAXIMUM_VOTE_VALUE,
        no: 0n,
      }

      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPowerLeaf)

      // hardhat default private key
      const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
      const account = privateKeyToAccount(privateKey)
      const signature = await account.signMessage({ message: MESSAGE })
      const leaf = hashLeaf(account.address.toLowerCase(), votingPowerLeaf.toString())
      const leaves = [...LEAVES, leaf]
      const merkleProof = generateMerkleProof(0n, votingPowerLeaf, account.address.toLowerCase(), leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: votingPowerLeaf,
        slotAddress: account.address.toLowerCase(),
        isFirstVote: false,
      })

      // This should pass - vote equals balance
      const proof = await generateProof(inputs)
      const isValid = await verifyProof(proof)
      expect(isValid).toBe(true)
    })

    it('should throw when the vote is > balance', { timeout: 100000 }, async () => {
      const encodedVote = encodeVote(VOTE, VotingMode.GOVERNANCE, votingPower)

      // hardhat default private key
      const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
      const account = privateKeyToAccount(privateKey)
      const signature = await account.signMessage({ message: MESSAGE })
      const leaf = hashLeaf(account.address.toLowerCase(), votingPowerLeaf.toString())
      const leaves = [...LEAVES, leaf]
      const merkleProof = generateMerkleProof(0n, votingPowerLeaf, account.address.toLowerCase(), leaves)

      const inputs = await encryptVoteAndGenerateCRISPInputs({
        encodedVote,
        publicKey,
        previousCiphertext,
        signature,
        message: MESSAGE,
        merkleData: merkleProof,
        balance: votingPowerLeaf,
        slotAddress: account.address.toLowerCase(),
        isFirstVote: false,
      })

      // set balance to 0
      inputs.balance = '0'

      await expect(getCircuitOutputValue(inputs)).rejects.toThrow()
    })
  })
})
