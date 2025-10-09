// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { describe, expect, it } from 'vitest'

import { getTreeData } from '../src/token'

// @notice To run these tests you will need to have an instance of CRISP running locally
describe('Token data fetching', () => {
  const serverUrl = 'http://localhost:4000'
  it('should fetch token data from the CRISP server', async () => {
    const data = await getTreeData(serverUrl, 0)
    expect(data.length).toBeGreaterThan(0)
  })
})
