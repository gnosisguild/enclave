// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, it, expect } from 'vitest'
import { BfvProtocolParams, type ProtocolParams } from '@enclave-e3/sdk'

import { encodeVote, validateVote } from '../src/vote'
import { VotingMode } from '../src/types'

describe('Vote', () => {
  describe('encodeVote', () => {
    const vote = { yes: 10n, no: 0n }
    it('should work for valid votes', () => {
      const encoded = encodeVote(vote, VotingMode.GOVERNANCE, BfvProtocolParams.BFV_NORMAL)
      expect(encoded.length).toBe(BfvProtocolParams.BFV_NORMAL.degree)
    })
    it('should work with small moduli', () => {
      const params: ProtocolParams = {
        degree: 10,
        // irrelevant
        plaintextModulus: 0n,
        moduli: 0n,
      }
      const encoded = encodeVote(vote, VotingMode.GOVERNANCE, params)
      expect(encoded.length).toBe(params.degree)

      // 01010 = 10
      // 00000 = 0
      expect(encoded).toEqual(['0', '1', '0', '1', '0', '0', '0', '0', '0', '0'])
    })
  })
  describe('validateVote', () => {
    const validVote = { yes: 10n, no: 0n }
    const invalidVote = { yes: 5n, no: 5n }

    it('should throw an error for invalid GOVERNANCE votes', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, invalidVote)
      }).toThrow('Invalid vote for GOVERNANCE mode: cannot spread votes between options')
    })
    it('should work for valid GOVERNANCE votes', () => {
      expect(() => {
        validateVote(VotingMode.GOVERNANCE, validVote)
      }).not.toThrow()
    })
  })
})
