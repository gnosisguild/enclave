// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, it, expect } from 'vitest'
import { BfvProtocolParams, type ProtocolParams } from '@enclave-e3/sdk'

import { decodeTally, encodeVote, validateVote } from '../src/vote'
import { VotingMode } from '../src/types'

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
  })
})
