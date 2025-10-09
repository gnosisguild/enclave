// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, expect, it } from 'vitest'

import { getTreeData } from '../src/token'
import { CRISP_SERVER_URL } from './constants'

// @notice To run these tests you will need to have an instance of CRISP running locally
describe('Token data fetching', () => {
  it('should fetch token data from the CRISP server', async () => {
    const data = await getTreeData(CRISP_SERVER_URL, 0)
    expect(data.length).toBeGreaterThan(0)
  })
})
