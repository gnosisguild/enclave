// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import fs from 'fs/promises'
import path from 'path'
import { describe, it, expect, beforeAll } from 'vitest'
import { BfvProtocolParams, type ProtocolParams } from '@enclave-e3/sdk'

import { calculateValidIndicesForPlaintext, decodeTally, encodeVote, encryptVoteAndGenerateCRISPInputs, validateVote } from '../src/vote'
import { VotingMode } from '../src/types'
import { MAXIMUM_VOTE_VALUE } from '../src'

describe('Vote', () => {
  const votingPower = 10n
  describe('encodeVote', () => {
    const vote = { yes: 10n, no: 0n }
    it('should work for valid votes', () => {
      const encoded = encodeVote(vote, VotingMode.GOVERNANCE, BfvProtocolParams.BFV_NORMAL, votingPower)
      expect(encoded.length).toBe(BfvProtocolParams.BFV_NORMAL.degree)
    })
    it('should work with small moduli', () => {
      const params: ProtocolParams = {
        degree: 10,
        // irrelevant
        plaintextModulus: 0n,
        moduli: 0n,
      }
      const encoded = encodeVote(vote, VotingMode.GOVERNANCE, params, votingPower)
      expect(encoded.length).toBe(params.degree)

      // 01010 = 10
      // 00000 = 0
      expect(encoded).toEqual(['0', '1', '0', '1', '0', '0', '0', '0', '0', '0'])
    })
  })

  describe('decode tally', () => {
    it('should decode correctly', () => {
      const tally = ['0', '2', '0', '1', '0', '0', '0', '0', '0', '0']

      const decoded = decodeTally(tally, VotingMode.GOVERNANCE)
      expect(decoded.yes).toBe(18n)
      expect(decoded.no).toBe(0n)
    })
  })

  describe('validateVote', () => {
    const validVote = { yes: 10n, no: 0n }
    const invalidVote = { yes: 5n, no: 5n }

    const votingPower = 10n

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
    it('should throw when vote are greater than the voting power available', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, { yes: 11n, no: 0n }, votingPower)
      }).toThrow('Invalid vote for GOVERNANCE mode: vote exceeds voting power')
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
    const vote = { yes: 10n, no: 0n }
    const votingPower = 10n

    let publicKey: Uint8Array

    beforeAll(async () => {
      const buffer = await fs.readFile(path.resolve(__dirname, './fixtures/pubkey.bin'))

      publicKey = Uint8Array.from(buffer)
    })

    it('should encrypt a vote and generate the circuit inputs', async () => {
      const encodedVote = encodeVote(vote, VotingMode.GOVERNANCE, BfvProtocolParams.BFV_NORMAL, votingPower)
      const encryptedData = await encryptVoteAndGenerateCRISPInputs(encodedVote, publicKey)

      expect(encryptedData.encryptedVote).toBeInstanceOf(Uint8Array)
      expect(encryptedData.circuitInputs).toBeInstanceOf(Object)
    })
  })
})
