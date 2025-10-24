// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, expect, it } from 'vitest'

import { getRoundDetails, getRoundTokenDetails } from '../src/state'
import { CRISP_SERVER_URL } from './constants'
import { zeroAddress } from 'viem'

describe('State', () => {
  describe('getRoundDetails', () => {
    it('should get the state for a given e3Id from the CRISP server', async () => {
      const state = await getRoundDetails(CRISP_SERVER_URL, 0)
      expect(state).toBeDefined()
    })
  })

  describe('getTokenDetails', () => {
    it('should return the details of the token for a given e3Id from the CRISP server', async () => {
      const tokenDetails = await getRoundTokenDetails(CRISP_SERVER_URL, 0)
      expect(tokenDetails.tokenAddress).not.toBe(zeroAddress)
      expect(tokenDetails.threshold).toBeGreaterThan(0)
      expect(tokenDetails.snapshotBlock).toBeGreaterThan(0)
    })
  })
})
